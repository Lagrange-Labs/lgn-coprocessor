use anyhow::{bail, Context};
use bytes::Bytes;
use checksums::ops::{compare_hashes, create_hashes, read_hashes, write_hash_comparison_results};
use checksums::Error;
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use tracing::{debug, error, info};

pub struct ParamsLoader;

// Could make configurable but 3600 should be enough
const HTTP_TIMEOUT: u64 = 3600;
const DOWNLOAD_MAX_RETRIES: u8 = 3;

impl ParamsLoader {
    pub fn prepare_bincode<P: for<'a> serde::de::Deserialize<'a>>(
        base_url: &str,
        base_dir: &str,
        file_name: &str,
        checksum_expected_local_path: &str,
        skip_checksum: bool,
        skip_store: bool,
    ) -> anyhow::Result<P> {
        fs::create_dir_all(base_dir).context("Failed to create directory")?;

        let file = format!("{base_dir}/{file_name}");
        info!(
            "Checking if params are on local storage and have the right checksum: {}",
            file
        );
        let mut retries = 0;
        loop {
            debug!("checksum attempt number:  {:?}", retries);
            if retries >= DOWNLOAD_MAX_RETRIES {
                bail!("Downloading file {:?} failed", file);
            }
            let result = Self::verify_file_checksum(
                file_name,
                &file,
                checksum_expected_local_path,
                skip_checksum,
            );

            match result {
                Ok(true) => {
                    info!("Loading params from local storage {:?}", file);
                    let file = File::open(&file);
                    let reader = std::io::BufReader::new(file.unwrap());

                    return bincode::deserialize_from(reader).map_err(Into::into);
                }
                _ => {
                    info!("public params are not locally stored yet, or checksum mismatch");
                    retries += 1;

                    let params = Self::download_file(base_url, file_name)?;
                    if skip_checksum {
                        info!(
                            "skipping checksum and loading file from local storage {:?}",
                            file
                        );
                        let file = File::open(&file);
                        let reader = std::io::BufReader::new(file.unwrap());
                        return bincode::deserialize_from(reader).map_err(Into::into);
                    }
                    if !skip_store {
                        Self::store_file(&file, &params)
                            .context("Failed to store params to local storage")?;
                    }
                }
            }
        }
    }

    pub fn prepare_raw(
        base_url: &str,
        base_dir: &str,
        file_name: &str,
        checksum_expected_local_path: &str,
        skip_store: bool,
    ) -> anyhow::Result<Bytes> {
        std::fs::create_dir_all(base_dir).context("Failed to create directory")?;

        let file = format!("{base_dir}/{file_name}");

        let _ = Self::verify_file_checksum(file_name, &file, checksum_expected_local_path, false);

        match File::open(&file) {
            Ok(file) => Ok(Self::read_file(file)?),
            Err(err) => {
                info!("Failed to load params from local storage: {err}");

                let params = Self::download_file(base_url, file_name)?;
                if !skip_store {
                    Self::store_file(&file, &params)
                        .context("Failed to store params to local storage")?;
                }
                Ok(params)
            }
        }
    }

    fn download_file(base_url: &str, file_name: &str) -> anyhow::Result<Bytes> {
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
        Ok(params)
    }
    fn verify_file_checksum(
        file_name: &str,
        file: &str,
        checksum_expected_local_path: &str,
        skip_checksum: bool,
    ) -> anyhow::Result<bool> {
        // checking if file exists
        if File::open(file).is_err() {
            if let Err(err) = fs::remove_file(Path::new(file)) {
                debug!("non existing file {}: {}", file, err);
            }
            return Ok(false);
        }
        debug!("Computing file hash for: {:?}", file);
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

        debug!("Computed hashes: {:?}", computed_hashes);
        let computed_hash = computed_hashes
            .iter()
            .map(|hash| (file_name.to_owned(), hash.1.to_owned()))
            .collect();
        debug!("Computed hash: {:?}", computed_hash);

        let expected_hashes_file = Path::new(&checksum_expected_local_path);
        let expected_hashes = read_hashes(
            &mut std::io::stderr(),
            &("output".to_string(), expected_hashes_file.to_path_buf()),
        );
        debug!(
            "expected hashes from: {:?} is {:?}",
            expected_hashes_file, expected_hashes
        );

        //let expected_hash = expected_hashes.clone().unwrap().get_key_value(file);
        let expected_hash = expected_hashes
            .unwrap()
            .iter()
            .filter(|hash| hash.0 == file_name)
            .map(|hash| (hash.0.to_owned(), hash.1.to_owned()))
            .collect();
        debug!("expected_hash: {:?} ", expected_hash);
        let compare_hashes = compare_hashes("compare_hashes", computed_hash, expected_hash);
        debug!("compare hashes: {:?} ", compare_hashes);

        let result = write_hash_comparison_results(
            &mut std::io::stdout(),
            &mut std::io::stderr(),
            compare_hashes.clone(),
        );
        debug!("checksum result: {:?} ", result);

        match result {
            Error::NoError => {
                // Test result no error
                info!("Checksum is successful");
                Ok(true)
            }

            _ => {
                if skip_checksum {
                    return Ok(false);
                }
                if let Err(err) = fs::remove_file(Path::new(file)) {
                    error!("Error deleting file {}: {}", file, err);
                }
                Ok(false)
            }
        }
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
}
