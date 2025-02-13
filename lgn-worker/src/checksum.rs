use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::bail;
use anyhow::Context;
use checksums::ops::compare_hashes;
use checksums::ops::create_hashes;
use checksums::ops::read_hashes;
use checksums::ops::write_hash_comparison_results;
use checksums::ops::write_hashes;
use checksums::ops::CompareFileResult;
use checksums::Error;
use reqwest::IntoUrl;
use tracing::debug;
use tracing::error;
use tracing::info;

pub(crate) fn verify_directory_checksums(
    dir: impl AsRef<OsStr> + Debug,
    expected_checksums_file: impl AsRef<Path>,
) -> anyhow::Result<()> {
    debug!("Computing hashes from: {:?}", dir);
    let computed_hashes = create_hashes(
        Path::new(dir.as_ref()),
        BTreeSet::new(),
        checksums::Algorithm::BLAKE3,
        None,
        true,
        3,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    debug!("Computed hashes: {:?}", computed_hashes);
    write_hashes(
        &(
            "output".to_string(),
            Path::new("public_params.hash").to_path_buf(),
        ),
        checksums::Algorithm::BLAKE3,
        computed_hashes.clone(),
    );
    let expected_hashes_file = Path::new(expected_checksums_file.as_ref());
    let expected_hashes = read_hashes(
        &mut std::io::stderr(),
        &("output".to_string(), expected_hashes_file.to_path_buf()),
    );
    debug!(
        "expected hashes from: {:?} is {:?}",
        expected_hashes_file, expected_hashes
    );
    let compare_hashes =
        compare_hashes("compare_hashes", computed_hashes, expected_hashes.unwrap());
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
        },
        Error::NFilesDiffer(count) => {
            if let Ok((_, file_results)) = &compare_hashes {
                let file_differs: Vec<&CompareFileResult> = file_results
                    .iter()
                    .filter(|f| matches!(f, CompareFileResult::FileDiffers { .. }))
                    .collect();

                for file_differ in file_differs {
                    if let CompareFileResult::FileDiffers { file, .. } = file_differ {
                        info!("File did not match the checksum. Deleting File {} ", file);
                        // This will only delete the file where the checksum has failed
                        if let Err(err) = fs::remove_file(Path::new(dir.as_ref()).join(file)) {
                            error!("Error deleting file {}: {}", file, err);
                        }
                    }
                }
            } else {
                error!("Failed to get file comparison results");
            }
            bail!("{} files do not match", count);
        },
        _ => {
            error!("Checksum failure: {:?}", result)
        },
    }

    Ok(())
}

pub(crate) fn fetch_checksum_file(
    url: impl IntoUrl,
    local_path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let response = reqwest::blocking::get(url)
        .context("Failed to fetch checksum file")?
        .text()
        .context("Failed to read response text")?;

    let mut file = File::create(local_path).context("Failed to create local checksum file")?;
    file.write_all(response.as_bytes())
        .context("Failed to write checksum file")?;

    Ok(())
}
