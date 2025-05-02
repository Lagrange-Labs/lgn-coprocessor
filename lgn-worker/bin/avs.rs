use std::env;
use std::fmt::Debug;
use std::fs;
use std::io::IsTerminal;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::bail;
use anyhow::Result;
use clap::Args;
use clap::Parser;
use ethers::providers::Http;
use ethers::providers::Provider;
use ethers::signers::Signer;
use ethers::signers::Wallet;
use lgn_worker::avs::contract::calculate_registration_digest_hash;
use lgn_worker::avs::contract::deregister_operator;
use lgn_worker::avs::contract::is_operator;
use lgn_worker::avs::contract::register_operator;
use lgn_worker::avs::contract::Client;
use lgn_worker::avs::contract::Network;
use lgn_worker::avs::public_key::PublicKey;
use lgn_worker::avs::utils::expiry_timestamp;
use lgn_worker::avs::utils::read_keystore;
use lgn_worker::avs::utils::read_password;
use lgn_worker::avs::utils::salt;
use lgn_worker::avs::utils::sign_hash;
use rand::thread_rng;
use tracing::debug;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
enum Cli {
    /// Generate a new Lagrange key
    NewKey(NewKey),
    /// Register to the AVS service
    Register(Register),
    /// De-register from the AVS service
    DeRegister(DeRegister),
}

const DEFAULT_ETH_KEYSTORE: &str = "eth_keystore.json";
const DEFAULT_LAGR_KEYSTORE: &str = "lagr_keystore.json";

const ETH_PRIVATE_KEY_ENV_VAR: &str = "AVS__ETH_PRIVATE_KEY";

const ETH_KEYSTORE_ENV_VAR: &str = "AVS__ETH_KEYSTORE";
const ETH_PWD_ENV_VAR: &str = "AVS__ETH_PWD";

const LAGR_KEYSTORE_ENV_VAR: &str = "AVS__LAGR_KEYSTORE";
const LAGR_PWD_ENV_VAR: &str = "AVS__LAGR_PWD";

#[derive(Args, Debug)]
struct NewKey {
    /// File path to load the Lagrange keystore for registering to the AVS service,
    /// could set ENV AVS__LAGR_PWD for password or input following the prompt.
    #[clap(short, long, env = LAGR_KEYSTORE_ENV_VAR, default_value_t = { DEFAULT_LAGR_KEYSTORE.to_string() })]
    lagr_keystore: String,
}

impl NewKey {
    /// generates a random wallet, encrypt it and saves it on disk
    pub fn run(&self) -> Result<()> {
        let password = read_password(LAGR_PWD_ENV_VAR, "Input password for Lagrange key: ")?;

        let path = Path::new(&self.lagr_keystore);
        let dir = path.parent().unwrap_or(Path::new(""));
        fs::create_dir_all(dir)?;
        let filename = path.file_name().and_then(|s| s.to_str());

        let (wallet, _) = Wallet::new_keystore(dir, &mut thread_rng(), password, filename)?;
        info!("new Lagrange keystore stored under {}", self.lagr_keystore);
        let public_key: PublicKey = wallet.signer().verifying_key().into();
        info!("public key: {}", public_key.to_hex());
        Ok(())
    }
}

#[derive(Args, Debug)]
struct Register {
    /// URL for RPC requests
    #[clap(short, long, env)]
    rpc_url: String,
    /// File path to load the main AVS keystore for signing, could set ENV
    /// AVS__AVS_PWD for password or input following the prompt.
    /// If the ENV AVS_SECRET_KEY is set as the main AVS private key, this
    /// argument will be ignored.
    #[clap(short, long, env = ETH_KEYSTORE_ENV_VAR, default_value_t = { DEFAULT_ETH_KEYSTORE.to_string() })]
    eth_keystore: String,
    /// File path to load the Lagrange keystore for registering to the AVS service,
    /// could set ENV AVS__LAGR_PWD for password or input following the prompt.
    #[clap(short, long, env = LAGR_KEYSTORE_ENV_VAR, default_value_t = { DEFAULT_LAGR_KEYSTORE.to_string() })]
    lagr_keystore: String,

    #[clap(short, long, env, default_value_t, value_enum)]
    network: Network,
}

impl Register {
    /// <https://github.com/Lagrange-Labs/client-cli/blob/develop/utils/chainops.go#L80-L84>
    async fn run(&self) -> Result<()> {
        info!("Running operation on {}", self.network.describe());
        // Restore the main AVS key, try to check if ENV AVS_SECRET_KEY is set.
        let main_wallet = env::var(ETH_PRIVATE_KEY_ENV_VAR).map_or_else(
            |_| {
                // Restore the main AVS key for key-store.
                let password = read_password(ETH_PWD_ENV_VAR, "Input password for main AVS key: ")?;
                read_keystore(&self.eth_keystore, password)
            },
            |main_key| {
                // Restore the main AVS key for the secret key.
                Ok(Wallet::from_str(&main_key)?)
            },
        )?;
        let main_wallet = main_wallet.with_chain_id(self.network.chain_id());

        // Restore the Lagrange key for registering to the AVS service.
        let password = read_password(LAGR_PWD_ENV_VAR, "Input password for Lagrange key: ")?;
        let lagrange_wallet = read_keystore(&self.lagr_keystore, password)?;

        let operator = main_wallet.address();
        let salt = salt();

        let provider = Arc::new(Provider::<Http>::try_from(&self.rpc_url)?);
        let expiry = expiry_timestamp(&provider).await?;

        debug!(
            "operator = {}, salt = 0x{}, expiry = {}",
            operator,
            hex::encode(salt),
            expiry
        );

        // Call the AVSDirectory contract to calculate the digest hash.
        let digest_hash = calculate_registration_digest_hash(
            &self.network,
            provider.clone(),
            operator,
            salt,
            expiry,
        )
        .await?;

        debug!("digest_hash = 0x{}", hex::encode(digest_hash));

        // Sign the hash.
        let client = Arc::new(Client::new(provider.clone(), main_wallet));
        let signature = sign_hash(client.signer(), digest_hash)?;

        let public_key = lagrange_wallet.signer().verifying_key().into();

        debug!(
            "signature = 0x{}, public_key = {:?}",
            hex::encode(&signature),
            public_key,
        );

        let is_operator = is_operator(&self.network, provider, operator).await?;
        if !is_operator {
            bail!("
Please register the main key as an operator of EigenLayer first:
https://docs.eigenlayer.xyz/eigenlayer/operator-guides/operator-installation#operator-configuration-and-registration
            ");
        }

        // Call the ZKMRStakeRegistry contract to register the operator.
        register_operator(&self.network, client, public_key, salt, expiry, signature).await?;

        info!("Operator {} successfully registered", operator);

        Ok(())
    }
}
#[derive(Args, Debug)]
struct DeRegister {
    /// URL for blockchain RPC requests.
    #[clap(short, long, env)]
    rpc_url: String,
    /// File path to load the main AVS keystore for signing, could set ENV
    /// AVS__AVS_PWD for password or input following the prompt.
    ///
    /// If the ENV AVS_SECRET_KEY is set as the main AVS private key, this
    /// argument will be ignored.
    #[clap(short, long, env = ETH_KEYSTORE_ENV_VAR, default_value_t = { DEFAULT_ETH_KEYSTORE.to_string() })]
    eth_keystore: String,
    /// File path to load the Lagrange keystore for registering to the AVS service,
    /// could set ENV AVS__LAGR_PWD for password or input following the prompt.
    #[clap(short, long, env = LAGR_KEYSTORE_ENV_VAR, default_value_t = { DEFAULT_LAGR_KEYSTORE.to_string() })]
    lagr_keystore: String,

    #[clap(short, long, env, default_value_t, value_enum)]
    network: Network,
}

impl DeRegister {
    async fn run(&self) -> Result<()> {
        info!("Running operation on {}", self.network.describe());
        // Restore the main AVS key, try to check if ENV AVS_SECRET_KEY is set.
        let main_wallet = env::var(ETH_PRIVATE_KEY_ENV_VAR).map_or_else(
            |_| {
                // Restore the main AVS key for key-store.
                let password = read_password(ETH_PWD_ENV_VAR, "Input password for main AVS key: ")?;
                read_keystore(&self.eth_keystore, password)
            },
            |main_key| {
                // Restore the main AVS key for the secret key.
                Ok(Wallet::from_str(&main_key)?)
            },
        )?;
        let main_wallet = main_wallet.with_chain_id(self.network.chain_id());
        let operator = main_wallet.address();
        info!("deregistering operator at address {}", operator);
        let provider = Arc::new(Provider::<Http>::try_from(&self.rpc_url)?);
        let client = Arc::new(Client::new(provider.clone(), main_wallet.clone()));

        let is_operator = is_operator(&self.network, provider, operator).await?;
        if !is_operator {
            bail!("Address {} does not belong to a known operator", operator);
        }

        deregister_operator(&self.network, client).await?;
        info!("Successfully de-registered operator {}", operator);

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::io::stdout().is_terminal() {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    } else {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    }

    let cli = Cli::parse();
    info!("Running {cli:?}");

    match cli {
        Cli::NewKey(new_key) => new_key.run(),
        Cli::Register(register) => register.run().await,
        Cli::DeRegister(deregister) => deregister.run().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    // It seems that this needs to be manually tested, as it requires user input
    fn test_new_key_generation() {
        let new_key = NewKey {
            lagr_keystore: "zkmr_store.json".to_string(),
        };

        new_key.run().unwrap();
    }
}
