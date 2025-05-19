use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use anyhow::bail;
use anyhow::ensure;
use anyhow::Context;
use bytes::Bytes;
use ethers::providers::StreamExt;
use reqwest::StatusCode;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::error;
use tracing::info;
use tracing::warn;

/// The filename of params checksum hashes
pub const PARAMS_CHECKSUM_FILENAME: &str = "public_params.hash";

/// The timeout is applied from when the request starts connecting until the response body has
/// finished.
const HTTP_TIMEOUT_MILLIS: u64 = 3_600_000;

/// The timeout applies to each read operation, and resets after a successful read.
const READ_TIMEOUT_MILLIS: u64 = 30_000;

/// Connection timeout.
const CONNECT_TIMEOUT_MILLIS: u64 = 5_000;

/// How many times param download should be retried.
const DOWNLOAD_BACKOFF_RETRIES: u8 = 3;

/// Minimum wait time for the exponential backoff.
const DOWNLOAD_BACKOFF_MIN_MILLIS: u64 = 100;

/// Maximum wait time for the exponential backoff.
const DOWNLOAD_BACKOFF_MAX_MILLIS: u64 = 10_000;

/// Ratio to convert byte to megabytes.
const BYTES_TO_MEGABYTES_USIZE: usize = 1024 * 1024;
const BYTES_TO_MEGABYTES_U64: u64 = 1024 * 1024;

/// Download and verify `file_name`.
///
/// This function will download `file_name` if necessary and checksum its contents.
///
/// Note: The checksum only checks the integrity of the file's content, it does
/// not guarantee the file is the correct one. That is to say, the file's content
/// can pass the checksum check, but the binary content may be incorrect and represent
/// data for a different version.
pub async fn download_and_checksum(
    base_url: &str,
    param_dir: &str,
    file_name: &str,
    checksums: &HashMap<String, blake3::Hash>,
) -> anyhow::Result<Bytes> {
    let mut filepath = PathBuf::from(param_dir);
    filepath.push(file_name);

    let param_dir = filepath
        .parent()
        .with_context(|| format!("Param directory can not be empty. param_dir: {}", param_dir))?;

    std::fs::create_dir_all(param_dir).with_context(|| {
        format!(
            "Failed to create directory for parameter files. param_dir: {}",
            param_dir.display(),
        )
    })?;

    let expected_checksum = checksums
        .get(file_name)
        .with_context(|| format!("Missing checksum. file_name: {}", file_name))?;

    let mut file = File::options()
        .read(true)
        .append(true)
        .truncate(false)
        .create(true)
        .open(&filepath)
        .await
        .with_context(|| format!("Failed to create file. filepath: {}", filepath.display()))?;

    let mut hasher = blake3::Hasher::new();
    let mut buf = std::fs::read(&filepath)
        .with_context(|| format!("Reading file failed. filepath: {}", filepath.display()))?;
    hasher.update_rayon(&buf);

    if *expected_checksum == hasher.finalize() {
        info!(
            "Found file matching checksum, skipping download. filepath: {}",
            filepath.display()
        );
        return Ok(Bytes::from(buf));
    }

    let fileurl = format!("{base_url}/{file_name}");

    let min = std::time::Duration::from_millis(DOWNLOAD_BACKOFF_MIN_MILLIS);
    let max = std::time::Duration::from_millis(DOWNLOAD_BACKOFF_MAX_MILLIS);
    let backoff = exponential_backoff::Backoff::new(DOWNLOAD_BACKOFF_RETRIES.into(), min, max);

    let client = reqwest::Client::builder()
        .referer(false)
        .timeout(Duration::from_secs(HTTP_TIMEOUT_MILLIS))
        .read_timeout(Duration::from_secs(READ_TIMEOUT_MILLIS))
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_MILLIS))
        .build()?;

    for (retry, duration) in backoff.iter().enumerate() {
        let result = resume_download(
            &mut buf,
            base_url,
            &mut file,
            &filepath,
            &fileurl,
            retry,
            &client,
            &mut hasher,
            expected_checksum,
        )
        .await;

        match result {
            Ok(()) => {
                info!(
                    "Params loaded. filepath: {} size: {}MiB",
                    filepath.display(),
                    buf.len() / BYTES_TO_MEGABYTES_USIZE,
                );
                return Ok(Bytes::from(buf));
            },
            Err(err) => {
                if let Some(duration) = duration {
                    warn!(
                        "Params download failed, retrying. filepath: {} err: {:?}",
                        filepath.display(),
                        err,
                    );
                    tokio::time::sleep(duration).await;
                }
            },
        };
    }

    bail!(
        "Download failed after retries. filepath: {} retries: {}",
        filepath.display(),
        DOWNLOAD_BACKOFF_RETRIES,
    );
}

#[allow(clippy::too_many_arguments)]
async fn resume_download(
    buf: &mut Vec<u8>,
    base_url: &str,
    file: &mut File,
    filepath: &Path,
    fileurl: &str,
    retry: usize,
    client: &reqwest::Client,
    hasher: &mut blake3::Hasher,
    expected_checksum: &blake3::Hash,
) -> anyhow::Result<()> {
    let metadata = file.metadata().await?;

    let response = client
        .get(fileurl)
        .header("Range", format!("bytes={}-", metadata.len()))
        .send()
        .await?;

    let length =
        response
            .headers()
            .get("Content-Length")
            .map_or(Ok(0u64), |v| -> anyhow::Result<u64> {
                let as_ascii = v.to_str()?;
                let parsed = u64::from_str(as_ascii)?;
                Ok(parsed)
            })?;

    info!(
        "Downloading params. base_url: {} filepath: {} present: {}MiB download: {}MiB retry: {}",
        base_url,
        filepath.display(),
        metadata.len() / BYTES_TO_MEGABYTES_U64,
        length / BYTES_TO_MEGABYTES_U64,
        retry,
    );

    if response.status() == StatusCode::RANGE_NOT_SATISFIABLE {
        warn!(
            "Local file is bigger than remote, resetting length and checking checksum. filepath: {}",
            filepath.display(),
        );

        hasher.reset();
        buf.resize(
            length.try_into().expect("File size should fit in a usize"),
            0,
        );
        file.set_len(length).await?;
        hasher.update_rayon(buf);
    } else {
        ensure!(
            response.status().is_success(),
            "Requesting params failed. status: {} filepath: {}",
            response.status(),
            filepath.display(),
        );

        let mut stream = response.bytes_stream();
        while let Some(data) = stream.next().await {
            let data = data?;
            file.write_all(&data).await?;
            hasher.update_rayon(&data);
            buf.extend(data);
        }
    }

    let found_checksum = hasher.finalize();
    if found_checksum != *expected_checksum {
        error!(
            "Checksum failed, restarting download. checksum: {} expected: {} filepath: {}",
            found_checksum.to_hex(),
            expected_checksum.to_hex(),
            filepath.display(),
        );

        hasher.reset();
        buf.clear();
        file.set_len(0).await?;

        bail!(
            "Checksum failed, restarting download. checksum: {} expected: {} filepath: {}",
            found_checksum.to_hex(),
            expected_checksum.to_hex(),
            filepath.display(),
        );
    } else {
        info!("Downloaded file. filepath: {}", filepath.display());
        Ok(())
    }
}
