use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::ensure;
use anyhow::Context;
use bytes::Bytes;
use tracing::info;

/// The filename of params checksum hashes
pub const PARAMS_CHECKSUM_FILENAME: &str = "public_params.hash";

/// Could make configurable but 3600 should be enough
const HTTP_TIMEOUT: u64 = 3600;

/// How many times param download should be retried.
const DOWNLOAD_MAX_RETRIES: u8 = 3;

/// Read the given file `f`, and returns its content as well as its Blake3 checksum.
fn read_file_and_checksum(f: &Path) -> anyhow::Result<(Bytes, blake3::Hash)> {
    let bytes = std::fs::read(f).with_context(|| anyhow!("reading `{}`", f.display()))?;
    let mut hasher = blake3::Hasher::new();
    hasher.update_rayon(&bytes);
    let hash = hasher.finalize();
    Ok((bytes.into(), hash))
}

pub fn prepare_raw(
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
            "parameter file `{}` has no parent directory",
            local_param_filename.display()
        )
    })?;
    std::fs::create_dir_all(current_param_dir).with_context(|| {
        format!(
            "failed to create directory `{}`",
            current_param_dir.display()
        )
    })?;

    let expected_checksum = checksums
        .get(file_name)
        .with_context(|| anyhow!("no expected checksum for `{file_name}`"))?;

    // A file must be re-downloaded if the local file does not exist or if its checksum
    // mismatches.
    let mut local_file_bytes = None;
    let need_download =
        if !local_param_filename.exists() {
            info!("`{}` does not exist", local_param_filename.display());
            true
        } else {
            false
        } || read_file_and_checksum(&local_param_filename).map(|(bytes, found)| {
            local_file_bytes = Some(bytes);
            if *expected_checksum != found {
                info!(
                    "local file `{}` hash is {} ≠ {}",
                    local_param_filename.display(),
                    expected_checksum.to_hex(),
                    found.to_hex()
                );
            }
            *expected_checksum != found
        })?;

    let bytes = if need_download {
        let mut bytes = Bytes::default();

        // Attempt to download the params upd to DOWNLOAD_MAX_RETRIES, with exponential backoff.
        let min = std::time::Duration::from_millis(100);
        let max = std::time::Duration::from_secs(10);
        for duration in exponential_backoff::Backoff::new(DOWNLOAD_MAX_RETRIES.into(), min, max) {
            match download_file(base_url, file_name, expected_checksum) {
                Ok(content) => {
                    info!("writing content to `{}`", local_param_filename.display());
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
                        None => return err.with_context(|| anyhow!("downloading `{}`", file_name)),
                    }
                },
            }
        }
        bytes
    } else {
        // Here, we already know that the checksum match.
        info!(
            "loading `{}` from `{}`",
            file_name,
            local_param_filename.display()
        );

        local_file_bytes.unwrap()
    };

    info!("params loaded, size = {}MiB", bytes.len() / (1024 * 1024));

    Ok(bytes)
}

/// Download the content from `file_name` under `base_url`, ensuring that its checksum matches
/// the provided `expected_checksum`.
fn download_file(
    base_url: &str,
    file_name: &str,
    expected_checksum: &blake3::Hash,
) -> anyhow::Result<Bytes> {
    let file_url = format!("{base_url}/{file_name}");
    info!("downloading params from {}", file_url);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT))
        .build()
        .context("building reqwest client")?;

    let response = client
        .get(file_url)
        .send()
        .context("downloading params from remote")?;

    if !response.status().is_success() {
        bail!(
            "downloading params from remote: status = {}",
            response.status()
        );
    }

    let bytes = response.bytes().context("fetching params bytes")?;
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
