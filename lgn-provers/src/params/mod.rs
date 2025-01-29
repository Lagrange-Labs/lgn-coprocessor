use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use anyhow::ensure;
use anyhow::Context;
use anyhow::*;
use bytes::Bytes;
use checksums::ops::compare_hashes;
use checksums::ops::create_hashes;
use checksums::ops::read_hashes;
use checksums::ops::write_hash_comparison_results;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

/// The filename of params checksum hashes
pub const PARAMS_CHECKSUM_FILENAME: &str = "public_params.hash";

pub struct ParamsLoader;

// Could make configurable but 3600 should be enough
const HTTP_TIMEOUT: u64 = 3600;
const DOWNLOAD_MAX_RETRIES: u8 = 3;

/// Utility to open a file and format an error if it errors.
///
/// [std::fs::File::open] does not format the file name into its error, it only returns NotFound.
fn open(file_path: &PathBuf) -> anyhow::Result<File> {
    File::open(file_path).with_context(
        || {
            format!(
                "failed to open `{:?}`",
                file_path
            )
        },
    )
}

/// Utility to create a directory and format the error if it happens.
///
/// Returns an error if a normal file already exists and its name clash with the requested path.
fn create_dir_all(directories: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(directories).with_context(
        || {
            format!(
                "failed to create directories `{:?}`",
                directories
            )
        },
    )
}

impl ParamsLoader {
    pub fn prepare_raw(
        base_url: &str,
        base_dir: &str,
        file_name: &str,
        checksum_expected_local_path: &str,
        skip_checksum: bool,
        skip_store: bool,
    ) -> anyhow::Result<Bytes> {
        create_dir_all(base_dir)?;
        let mut file = PathBuf::from(base_dir);
        file.push(file_name);

        let mut retries = 0;
        loop {
            info!(
                "Checking params checksum. file_path: {:?} retries: {}",
                file, retries,
            );
            ensure!(
                retries < DOWNLOAD_MAX_RETRIES,
                "Downloading file {:?} failed",
                file
            );
            let result = if !skip_checksum {
                Self::verify_file_checksum(
                    file_name,
                    &file,
                    checksum_expected_local_path,
                )
            } else {
                Ok(true)
            };

            match result {
                Result::Ok(true) => {
                    info!(
                        "Loading params from local storage {:?}",
                        file
                    );

                    let file = open(&file)?;
                    let mut reader = std::io::BufReader::new(file);
                    let mut buffer = Vec::new();
                    reader
                        .read_to_end(&mut buffer)
                        .context("Failed to read params from local storage")?;

                    let bytes = Bytes::from(buffer);
                    info!(
                        "Loaded params of size in KB: {}",
                        bytes
                            .as_ref()
                            .len()
                            / 1024
                    );

                    return Ok(bytes);
                },

                _ => {
                    info!("public params are not locally stored yet, or checksum mismatch");
                    retries += 1;

                    let params = Self::download_file(
                        base_url,
                        file_name,
                    )?;
                    if !skip_store {
                        Self::store_file(
                            &file,
                            &params,
                        )
                        .context("Failed to store params to local storage")?;
                    }
                },
            }
        }
    }

    fn download_file(
        base_url: &str,
        file_name: &str,
    ) -> anyhow::Result<Bytes> {
        let file_url = format!("{base_url}/{file_name}");
        info!(
            "Downloading params from {}",
            file_url
        );

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT))
            .build()
            .context("Failed to build reqwest client")?;

        let response = client
            .get(file_url)
            .send()
            .context("Failed to download params from remote")?;

        if !response
            .status()
            .is_success()
        {
            anyhow::bail!(
                "Failed to download params from remote: {}",
                response.status()
            );
        }

        let params = response
            .bytes()
            .context("Failed to download params from remote")?;

        info!(
            "Downloaded params of size in KB: {}",
            params.len() / 1024
        );
        Ok(params)
    }

    fn verify_file_checksum(
        file_name: &str,
        file: &Path,
        checksum_expected_local_path: &str,
    ) -> anyhow::Result<bool> {
        // checking if file exists
        if File::open(file).is_err() {
            if let Err(err) = std::fs::remove_file(Path::new(file)) {
                debug!(
                    "non existing file {:?}: {}",
                    file, err
                );
            }
            return Ok(false);
        }

        let computed_hashes = create_hashes(
            Path::new(file),
            BTreeSet::new(),
            checksums::Algorithm::BLAKE3,
            None,
            true,
            3,
            &mut std::io::stdout(),
            &mut std::io::stderr(),
        );

        let computed_hash: BTreeMap<String, String> = computed_hashes
            .iter()
            .map(
                |hash| {
                    (
                        file_name.to_owned(),
                        hash.1
                            .to_owned(),
                    )
                },
            )
            .collect();

        let expected_hashes_file = Path::new(&checksum_expected_local_path);
        let expected_hashes = read_hashes(
            &mut std::io::stderr(),
            &(
                "output".to_string(),
                expected_hashes_file.to_path_buf(),
            ),
        )
        .unwrap_or_else(
            |_| {
                panic!(
                    "while comparing hashes to `{}`",
                    expected_hashes_file.display()
                )
            },
        );

        let expected_hash: BTreeMap<String, String> = expected_hashes
            .iter()
            .filter(|hash| hash.0 == file_name)
            .map(
                |hash| {
                    (
                        hash.0
                            .to_owned(),
                        hash.1
                            .to_owned(),
                    )
                },
            )
            .collect();
        let compare_hashes = compare_hashes(
            "compare_hashes",
            computed_hash.clone(),
            expected_hash.clone(),
        );

        let result = write_hash_comparison_results(
            &mut std::io::stdout(),
            &mut std::io::stderr(),
            compare_hashes.clone(),
        );

        debug!(
            "Checksum. file: {:?} computed_hashes: {:?} computed_hash: {:?} expected_hashes: {:?} expected_hash: {:?} compare_hashes: {:?}",
            file,
            computed_hashes,
            computed_hash,
            expected_hashes,
            expected_hash,
            compare_hashes,
        );

        match result {
            checksums::Error::NoError => {
                info!(
                    "Checksum is successful. file: {:?} computed_hash: {:?} expected_hash: {:?} compare_hashes: {:?}",
                    file,
                    computed_hash,
                    expected_hash,
                    compare_hashes,
                );
                Ok(true)
            },

            _ => {
                warn!(
                    "Checksum failed. file: {:?} computed_hash: {:?} expected_hash: {:?} compare_hashes: {:?}",
                    file,
                    computed_hash,
                    expected_hash,
                    compare_hashes,
                );
                if let Err(err) = std::fs::remove_file(Path::new(file)) {
                    error!(
                        "Error deleting file {:?}: {}",
                        file, err
                    );
                }
                Ok(false)
            },
        }
    }

    fn store_file(
        file: &Path,
        params: &Bytes,
    ) -> anyhow::Result<()> {
        info!(
            "Storing params to local storage: {:?}",
            file
        );

        if let Some(parent) = std::path::Path::new(file).parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create directories for local storage")?;
        }

        let file = File::create(file).context("Failed to create file for local storage")?;

        let mut buffer = std::io::BufWriter::new(file);
        buffer
            .write_all(params)
            .context("Failed to write params to local storage")?;
        buffer
            .flush()
            .context("Failed to flush params to local storage")?;
        info!("Stored params to local storage");
        Ok(())
    }
}
