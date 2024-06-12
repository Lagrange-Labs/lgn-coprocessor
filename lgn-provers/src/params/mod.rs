use anyhow::Context;
use bytes::Bytes;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use tracing::info;

pub struct ParamsLoader;

// Could make configurable but 3600 should be enough
const HTTP_TIMEOUT: u64 = 3600;

impl ParamsLoader {
    pub fn prepare_bincode<P: for<'a> serde::de::Deserialize<'a>>(
        base_url: &str,
        base_dir: &str,
        file_name: &str,
        checksum: &str,
        skip_store: bool,
    ) -> anyhow::Result<P> {
        std::fs::create_dir_all(base_dir).context("Failed to create directory")?;

        let file = format!("{base_dir}/{file_name}");
        info!("Checking if params are on local storage: {}", file);

        match File::open(&file) {
            Ok(fl) => {
                if Self::verify_checksum(&fl, checksum)? {
                    info!("Loading params from local storage");
                    let reader = std::io::BufReader::new(fl);
                    bincode::deserialize_from(reader).map_err(Into::into)
                } else {
                    info!("Checksum mismatch, downloading params again");
                    let params = Self::download_file(base_url, file_name, checksum)?;
                    if !skip_store {
                        Self::store_file(&file, &params)
                            .context("Failed to store params to local storage")?;
                    }

                    info!("Deserializing params");
                    let reader = std::io::BufReader::new(params.as_ref());
                    bincode::deserialize_from(reader).context("Failed to deserialize params")
                }
            }
            Err(_) => {
                info!("public params are not locally stored yet");

                let params = Self::download_file(base_url, file_name, checksum)?;
                if !skip_store {
                    Self::store_file(&file, &params)
                        .context("Failed to store params to local storage")?;
                }

                info!("Deserializing params");
                let reader = std::io::BufReader::new(params.as_ref());
                bincode::deserialize_from(reader).context("Failed to deserialize params")
            }
        }
    }

    pub fn prepare_raw(
        base_url: &str,
        base_dir: &str,
        file_name: &str,
        checksum: &str,
        skip_store: bool,
    ) -> anyhow::Result<Bytes> {
        std::fs::create_dir_all(base_dir).context("Failed to create directory")?;

        let file = format!("{base_dir}/{file_name}");
        info!("Checking if params are on local storage: {}", file);

        match File::open(&file) {
            Ok(file) => Ok(Self::read_file(file)?),
            Err(err) => {
                info!("Failed to load params from local storage: {err}");

                let params = Self::download_file(base_url, file_name, checksum)?;
                if !skip_store {
                    Self::store_file(&file, &params)
                        .context("Failed to store params to local storage")?;
                }
                Ok(params)
            }
        }
    }

    fn download_file(base_url: &str, file_name: &str, checksum: &str) -> anyhow::Result<Bytes> {
        let file_url = format!("{base_url}/{file_name}");
        info!("Downloading params from {}", file_url);

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT))
            .build()
            .context("Failed to build reqwest client")?;

        let response = client
            .get(file_url)
            .send()
            .context("Failed to download params from remote")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to download params from remote: {}",
                response.status()
            );
        }

        let params = response
            .bytes()
            .context("Failed to download params from remote")?;

        info!("Downloaded params of size in KB: {}", params.len() / 1024);

        if !Self::verify_checksum_from_bytes(&params, checksum)? {
            anyhow::bail!("Checksum mismatch for downloaded params");
        }

        Ok(params)
    }

    fn read_file(file: File) -> anyhow::Result<Bytes> {
        info!("Loading params from local storage {:?}", file);

        let mut reader = std::io::BufReader::new(file);
        let mut buffer = Vec::new();
        reader
            .read_to_end(&mut buffer)
            .context("Failed to read params from local storage")?;

        let bytes = Bytes::from(buffer);
        info!(
            "Loaded params of size in KB: {}",
            bytes.as_ref().len() / 1024
        );

        Ok(bytes)
    }

    fn store_file(file: &String, params: &Bytes) -> anyhow::Result<()> {
        info!("Storing params to local storage: {:?}", file);

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
    fn verify_checksum(file: &File, expected_checksum: &str) -> anyhow::Result<bool> {
        let mut reader = std::io::BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = Vec::new();
        reader
            .read_to_end(&mut buffer)
            .context("Failed to read file for checksum verification")?;
        hasher.update(&buffer);
        let actual_checksum = format!("{:x}", hasher.finalize());
        if actual_checksum != expected_checksum {
            info!(
                "Checksum mismatch: expected {}, but got {}",
                expected_checksum, actual_checksum
            );
        }
        Ok(actual_checksum == expected_checksum)
    }

    fn verify_checksum_from_bytes(bytes: &Bytes, expected_checksum: &str) -> anyhow::Result<bool> {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let actual_checksum = format!("{:x}", hasher.finalize());
        if actual_checksum != expected_checksum {
            info!(
                "Checksum mismatch: expected {}, but got {}",
                expected_checksum, actual_checksum
            );
        }
        Ok(actual_checksum == expected_checksum)
    }
}
