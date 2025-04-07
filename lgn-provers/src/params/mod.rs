use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::ensure;
use anyhow::Context;
use bytes::Bytes;
use ethers::providers::StreamExt;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncSeekExt;
use tokio::io::AsyncWriteExt;
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

    let current_param_dir = filepath.parent().with_context(|| {
        format!(
            "Parameter file has no parent directory. filepath: {}",
            filepath.display()
        )
    })?;

    std::fs::create_dir_all(current_param_dir).with_context(|| {
        format!(
            "Failed to create directory for parameter files. current_param_dir: {}",
            current_param_dir.display()
        )
    })?;

    let expected_checksum = checksums
        .get(file_name)
        .with_context(|| format!("Missing checksum. file_name: {}", file_name))?;

    let local_file_bytes = if filepath.exists() {
        let bytes = std::fs::read(&filepath)
            .with_context(|| format!("Reading file failed. filepath: {}", filepath.display()))?;
        let mut hasher = blake3::Hasher::new();
        hasher.update_rayon(&bytes);
        let checksum = hasher.finalize();
        if *expected_checksum != checksum {
            warn!(
                "Checksum mismatch. filepath: {} expected_checksum: {} checksum: {}",
                filepath.display(),
                expected_checksum.to_hex(),
                checksum.to_hex()
            );
            None
        } else {
            info!(
                "Found file with valid checksum, skipping download. filepath: {}",
                filepath.display()
            );
            Some(Bytes::from(bytes))
        }
    } else {
        None
    };

    let bytes = match local_file_bytes {
        None => {
            let mut bytes = Bytes::default();
            let file_url = format!("{base_url}/{file_name}");

            let min = std::time::Duration::from_millis(DOWNLOAD_BACKOFF_MIN_MILLIS);
            let max = std::time::Duration::from_millis(DOWNLOAD_BACKOFF_MAX_MILLIS);
            let backoff =
                exponential_backoff::Backoff::new(DOWNLOAD_BACKOFF_RETRIES.into(), min, max);

            for (retry, duration) in backoff.iter().enumerate() {
                info!(
                    "Downloading params. base_url: {} filepath: {} retry: {}",
                    base_url,
                    filepath.display(),
                    retry,
                );

                let mut file = File::options()
                    .read(true)
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(&filepath)
                    .await
                    .with_context(|| {
                        format!("Failed to create file. filepath: {}", filepath.display())
                    })?;

                match download_file(&file_url, expected_checksum, &mut file).await {
                    Ok(content) => {
                        info!("Downloaded file. filepath: {}", filepath.display());
                        bytes = content;
                        break;
                    },
                    err @ Err(_) => {
                        match duration {
                            Some(duration) => tokio::time::sleep(duration).await,
                            None => {
                                return err.with_context(|| {
                                    format!(
                                        "Download failed after retries. filepath: {} retries: {}",
                                        filepath.display(),
                                        retry
                                    )
                                })
                            },
                        }
                    },
                }
            }
            bytes
        },
        Some(bytes) => bytes,
    };

    info!(
        "Params loaded. filepath: {} size: {}MiB",
        filepath.display(),
        bytes.len() / (1024 * 1024),
    );

    Ok(bytes)
}

/// Download the content from `file_name` under `base_url`, ensuring that its checksum matches
/// the provided `expected_checksum`.
async fn download_file(
    file_url: &str,
    expected_checksum: &blake3::Hash,
    file: &mut File,
) -> anyhow::Result<Bytes> {
    let client = reqwest::Client::builder()
        .referer(false)
        .timeout(Duration::from_secs(HTTP_TIMEOUT_MILLIS))
        .read_timeout(Duration::from_secs(READ_TIMEOUT_MILLIS))
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_MILLIS))
        .build()?;

    let metadata = file.metadata().await?;
    let response = client
        .get(file_url)
        .header("Range", format!("bytes={}-", metadata.len()))
        .send()
        .await?;

    ensure!(
        response.status().is_success(),
        "downloading params from remote: status = {}",
        response.status()
    );

    let mut hasher = blake3::Hasher::new();

    let mut stream = response.bytes_stream();
    while let Some(data) = stream.next().await {
        let data = data?;
        file.write_all(&data).await?;
        hasher.update_rayon(&data);
    }

    let found_checksum = hasher.finalize();
    ensure!(
        found_checksum == *expected_checksum,
        "param checksum mismatch: {} ≠ {}",
        found_checksum.to_hex(),
        expected_checksum.to_hex()
    );

    let mut buffer = Vec::new();
    file.seek(SeekFrom::Start(0)).await?;
    file.read_to_end(&mut buffer).await?;

    Ok(buffer.into())
}
