use std::collections::HashMap;

use anyhow::anyhow;
use anyhow::Context;
use reqwest::IntoUrl;

/// Fetch the checksums stored at `url`, then parse them into a mapping from file name to Blake3
/// hash.
pub(crate) fn fetch_checksums(url: impl IntoUrl) -> anyhow::Result<HashMap<String, blake3::Hash>> {
    let url = url.into_url().context("parsing checksums URL")?;
    let mut r = HashMap::new();

    let response = reqwest::blocking::get(url.clone())
        .with_context(|| anyhow!("fetching checksum file at `{url}`"))?
        .text()
        .context("reading response text")?;

    for line in response.lines() {
        let mut line = line.split_whitespace();
        let source = line.next().context("no filename found")?;
        let hash_str = line.next().context("no hash found")?;
        match blake3::Hash::from_hex(hash_str) {
            Ok(hash) => {
                r.insert(source.to_owned(), hash);
            },
            Err(_) => {
                tracing::warn!("ignoring file `{source}` with invalid hash `{hash_str}`")
            },
        }
    }

    tracing::debug!("checksums: {r:?}");

    Ok(r)
}
