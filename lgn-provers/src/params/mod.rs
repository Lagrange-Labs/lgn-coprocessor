use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::ensure;
use anyhow::Context;
use bytes::Bytes;
use tracing::info;
use tracing::warn;

/// The filename of params checksum hashes
pub const PARAMS_CHECKSUM_FILENAME: &str = "public_params.hash";

/// Could make configurable but 3600 should be enough
const HTTP_TIMEOUT: u64 = 3600;

/// How many times param download should be retried.
const DOWNLOAD_MAX_RETRIES: u8 = 3;

/// Download and verify `file_name`.
///
/// This function will download `file_name` if necessary and checksum its contents.
///
/// Note: The checksum only checks the integrity of the file's content, it does
/// not guarantee the file is the correct one. That is to say, the file's content
/// can pass the checksum check, but the binary content may be incorrect and represent
/// data for a different version.
pub async fn prepare_raw(
    base_url: &str,
    param_dir: &str,
    file_name: &str,
    checksums: &HashMap<String, blake3::Hash>,
) -> anyhow::Result<Bytes> {
    let mut local_param_filename = PathBuf::from(param_dir);
    local_param_filename.push(file_name);
    // The parameter filename may be relative, thus it may be required to create a directory
    // deeper than just `param_dir`.
    let current_param_dir = local_param_filename.parent().ok_or_else(|| {
        anyhow!(
            "Parameter file has no parent directory. local_param_filename: {}",
            local_param_filename.display()
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
        .with_context(|| anyhow!("Missing checksum. file_name: {}", file_name))?;

    let local_file_bytes = if local_param_filename.exists() {
        let bytes = std::fs::read(&local_param_filename).with_context(|| {
            anyhow!(
                "Reading file failed. local_param_filename: {}",
                local_param_filename.display()
            )
        })?;
        let mut hasher = blake3::Hasher::new();
        hasher.update_rayon(&bytes);
        let checksum = hasher.finalize();
        if *expected_checksum != checksum {
            warn!(
                "Checksum mismatch. local_param_filename: {} expected_checksum: {} checksum: {}",
                local_param_filename.display(),
                expected_checksum.to_hex(),
                checksum.to_hex()
            );
            None
        } else {
            info!(
                "Found file with valid checksum, skipping download. local_param_filename: {}",
                local_param_filename.display()
            );
            Some(Bytes::from(bytes))
        }
    } else {
        None
    };

    let bytes = match local_file_bytes {
        None => {
            let mut bytes = Bytes::default();

            // Attempt to download the params up to DOWNLOAD_MAX_RETRIES, with exponential backoff.
            let min = std::time::Duration::from_millis(100);
            let max = std::time::Duration::from_secs(10);
            for (retry, duration) in
                exponential_backoff::Backoff::new(DOWNLOAD_MAX_RETRIES.into(), min, max)
                    .iter()
                    .enumerate()
            {
                info!(
                    "Downloading params. base_url: {} file_name: {} retry: {}",
                    base_url, file_name, retry,
                );

                match download_file(base_url, file_name, expected_checksum).await {
                    Ok(content) => {
                        info!(
                            "Downloaded file. local_param_filename: {}",
                            local_param_filename.display()
                        );
                        std::fs::File::create(&local_param_filename)
                            .context("creating param file")?
                            .write_all(&content)
                            .context("writing file content")?;
                        bytes = content;
                        break;
                    },
                    err @ Err(_) => {
                        match duration {
                            Some(duration) => std::thread::sleep(duration),
                            None => {
                                return err.with_context(|| {
                                    anyhow!(
                                        "Download failed after retries. file_name: {} retries: {}",
                                        file_name,
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
        "Params loaded. file_name: {} size: {}MiB",
        file_name,
        bytes.len() / (1024 * 1024),
    );

    Ok(bytes)
}

/// Download the content from `file_name` under `base_url`, ensuring that its checksum matches
/// the provided `expected_checksum`.
async fn download_file(
    base_url: &str,
    file_name: &str,
    expected_checksum: &blake3::Hash,
) -> anyhow::Result<Bytes> {
    let file_url = format!("{base_url}/{file_name}");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT))
        .build()
        .context("building reqwest client")?;

    let response = client
        .get(file_url)
        .send()
        .await
        .context("downloading params from remote")?;

    if !response.status().is_success() {
        bail!(
            "downloading params from remote: status = {}",
            response.status()
        );
    }

    let bytes = response.bytes().await.context("fetching params bytes")?;
    let mut hasher = blake3::Hasher::new();
    hasher.update_rayon(&bytes);
    let found_checksum = hasher.finalize();
    ensure!(
        found_checksum == *expected_checksum,
        "param checksum mismatch: {} ≠ {}",
        found_checksum.to_hex(),
        expected_checksum.to_hex()
    );
    Ok(bytes)
}
