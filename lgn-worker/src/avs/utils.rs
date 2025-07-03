use std::env;
use std::path::Path;

use alloy::eips::BlockNumberOrTag;
use alloy::primitives::U256;
use alloy::providers::Provider;
use alloy::providers::RootProvider;
use alloy::signers::local::LocalSigner;
use alloy::signers::local::PrivateKeySigner;
use alloy::signers::SignerSync;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use rand::thread_rng;
use rand::Rng;

/// The expiry time (5 minutes) is used in the eigen signature. It's not a
/// constant to check in the smart contract, so it could be any value, seems not
/// make sense.
const DEFAULT_EXPIRY_SECONDS: u64 = 300;

/// Get the expiry seconds.
/// <https://github.com/Lagrange-Labs/client-cli/blob/develop/utils/chainops.go#L85-L89>
pub async fn expiry_timestamp(provider: &RootProvider) -> Result<U256> {
    Ok(U256::from_be_slice(
        &(provider
            .get_block(BlockNumberOrTag::Latest.into())
            .await?
            .ok_or(anyhow!("Failed to get latest block"))?
            .header
            .inner
            .timestamp
            + DEFAULT_EXPIRY_SECONDS)
            .to_be_bytes(),
    ))
}

/// Read the password from input.
pub fn read_password(
    env_name: &str,
    prompt_msg: &str,
) -> Result<String> {
    match env::var(env_name) {
        Ok(password) if !password.is_empty() => Ok(password),
        _ => {
            if cfg!(test) {
                test_prompt_password(prompt_msg)
            } else {
                prompt_password(prompt_msg)
            }
        },
    }
}

/// Read the key-store from a file path with the specified password.
pub fn read_keystore<P: AsRef<Path>, S: AsRef<[u8]>>(
    key_path: P,
    password: S,
) -> Result<PrivateKeySigner> {
    let wallet = LocalSigner::decrypt_keystore(&key_path, password)
        .with_context(|| anyhow!("trying to open `{}`", key_path.as_ref().display()))?;

    Ok(wallet)
}

/// Generate the salt.
pub fn salt() -> [u8; 32] {
    // Generate 32 random bytes.
    thread_rng().gen()
}

/// Sign the hash.
/// <https://github.com/Lagrange-Labs/client-cli/blob/develop/utils/chainops.go#L94-L98>
pub fn sign_hash(
    wallet: &PrivateKeySigner,
    data: [u8; 32],
) -> Result<Vec<u8>> {
    // Sign the hash, and it has already added `v` with 27.
    // <https://github.com/gakonst/ethers-rs/blob/51fe937f6515689b17a3a83b74a05984ad3a7f11/ethers-signers/src/wallet/mod.rs#L152>
    Ok(wallet.sign_hash_sync(&data.into())?.as_bytes().to_vec())
}

/// Prompt to input password
fn prompt_password(prompt_msg: &str) -> Result<String> {
    Ok(rpassword::prompt_password(prompt_msg)?)
}

/// Prompt to input password for testing
fn test_prompt_password(prompt_msg: &str) -> Result<String> {
    use std::io::Cursor;

    let mut mock_input = Cursor::new("test-password\n".as_bytes().to_owned());
    let mut mock_output = Cursor::new(Vec::new());

    let password =
        rpassword::prompt_password_from_bufread(&mut mock_input, &mut mock_output, prompt_msg)?;

    Ok(password)
}
