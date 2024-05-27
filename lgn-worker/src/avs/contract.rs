pub use super::public_key::PublicKey;
use anyhow::{bail, Result};
use ethers::prelude::{abigen, Address, Http, Provider, SignerMiddleware, Wallet, U256};
use k256::ecdsa::SigningKey;
use serde::Serialize;
use std::sync::Arc;

/// ZKMR service manager address as an argument (avs) to call the contract
/// function `calculateOperatorAVSRegistrationDigestHash`
/// - currently same address for mainnet and holesky
const ZKMR_SERVICE_MANAGER_ADDR: &str = "0xf98D5De1014110C65c51b85Ea55f73863215CC10";

/// ZKMRStakeRegistry contract address
/// <https://github.com/Lagrange-Labs/lpn-relayer/blob/feat/avs-relay/src/config/chain.ts#L57>
/// - currently same address for mainnet and holesky
const ZKMR_STAKE_REGISTRY_ADDR: &str = "0xf724cDC7C40fd6B59590C624E8F0E5E3843b4BE4";

/// AVSDirectory contract address
/// from https://github.com/Layr-Labs/eigenlayer-contracts?tab=readme-ov-file#deployments
const MAINNET_AVS_DIRECTORY_ADDR: &str = "0x135dda560e946695d6f155dacafc6f1f25c1f5af";
const HOLESKY_AVS_DIRECTORY_ADDR: &str = "0x055733000064333CaDDbC92763c58BF0192fFeBf";

/// DelegationManager contract address
/// from https://github.com/Layr-Labs/eigenlayer-contracts?tab=readme-ov-file#deployments
const MAINNET_DELEGATION_MANAGER_ADDR: &str = "0x39053D51B77DC0d36036Fc1fCc8Cb819df8Ef37A";
const HOLESKY_DELEGATION_MANAGER_ADDR: &str = "0xA44151489861Fe9e3055d95adC98FbD462B948e7";

abigen!(
    AVSDirectory,
    "$CARGO_MANIFEST_DIR/abis/AVSDirectoryABI.json"
);
abigen!(
    DelegationManager,
    "$CARGO_MANIFEST_DIR/abis/DelegationManagerABI.json"
);
abigen!(
    ZKMRStakeRegistry,
    "$CARGO_MANIFEST_DIR/abis/ZKMRStakeRegistryABI.json"
);

#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    #[default]
    Mainnet,
    Holesky,
}

impl Network {
    pub fn describe(&self) -> String {
        match self {
            Network::Mainnet => "mainnet",
            Network::Holesky => "holesky",
        }
        .to_string()
    }

    pub fn chain_id(&self) -> u64 {
        match self {
            Network::Mainnet => 1,
            Network::Holesky => 17000u64,
        }
    }
    /// Returns the address of the lagrange registry necessary to register an operator
    /// on Lagrange Network AVS
    fn lagrange_registry_address(&self) -> Address {
        // match self {
        //     // keeping optionality to have different address if it comes up one day
        //     _ => ZKMR_STAKE_REGISTRY_ADDR.to_string(),
        // }
        ZKMR_STAKE_REGISTRY_ADDR
            .to_string()
            .parse()
            .expect("invalid registry address")
    }
    /// Returns the address of the service manager contract. Necessary input to
    /// compute the right avs digest hash for the registration signature.
    fn lagrange_service_manager_address(&self) -> Address {
        // match self {
        //     _ => ZKMR_SERVICE_MANAGER_ADDR.to_string(),
        // }
        ZKMR_SERVICE_MANAGER_ADDR
            .to_string()
            .parse()
            .expect("invalid service manager address")
    }
    /// Returns the delegation manager contract address necessary to ensure an operator is
    /// already registered or not yet.
    /// From https://github.com/Layr-Labs/eigenlayer-contracts?tab=readme-ov-file#deployments
    fn eigen_delegation_manager_address(&self) -> Address {
        match self {
            Network::Mainnet => MAINNET_DELEGATION_MANAGER_ADDR.to_string(),
            Network::Holesky => HOLESKY_DELEGATION_MANAGER_ADDR.to_string(),
        }
        .parse()
        .expect("invalid delegation manager address")
    }

    /// Returns the AVS directory contract address necessary to compute the AVS digest
    /// hash - since that hash is computed onchain as well during registration time.
    fn eigen_avs_directory(&self) -> Address {
        match self {
            Network::Mainnet => MAINNET_AVS_DIRECTORY_ADDR.to_string(),
            Network::Holesky => HOLESKY_AVS_DIRECTORY_ADDR.to_string(),
        }
        .parse()
        .expect("invalid contract avs directory address")
    }
}
pub type Client = SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>;

/// Call DelegationManager contract function `isOperator`
pub async fn is_operator(
    network: &Network,
    provider: Arc<Provider<Http>>,
    operator: Address,
) -> Result<bool> {
    let contract_address: Address = network.eigen_delegation_manager_address();
    let contract = DelegationManager::new(contract_address, provider);

    Ok(contract.is_operator(operator).call().await?)
}

/// Call AVSDirectory contract function `calculateOperatorAVSRegistrationDigestHash`
pub async fn calculate_registration_digest_hash(
    network: &Network,
    provider: Arc<Provider<Http>>,
    operator: Address,
    salt: [u8; 32],
    expiry: U256,
) -> Result<[u8; 32]> {
    let avs: Address = network.lagrange_service_manager_address();

    let contract_address: Address = network.eigen_avs_directory();
    let contract = AVSDirectory::new(contract_address, provider);

    let digest_hash = contract
        .calculate_operator_avs_registration_digest_hash(operator, avs, salt, expiry)
        .call()
        .await?;

    Ok(digest_hash)
}

/// Call ZKMRStakeRegistry contract function `registerOperator`
pub async fn register_operator(
    network: &Network,
    client: Arc<Client>,
    public_key: PublicKey,
    salt: [u8; 32],
    expiry: U256,
    signature: Vec<u8>,
) -> Result<()> {
    let operator_address = client.address();
    let contract_address: Address = network.lagrange_registry_address();
    let contract = ZKMRStakeRegistry::new(contract_address, client);

    let public_key = zkmr_stake_registry::PublicKey {
        x: public_key.x,
        y: public_key.y,
    };
    let signature = zkmr_stake_registry::SignatureWithSaltAndExpiry {
        expiry,
        salt,
        signature: signature.into(),
    };
    // we first check if we are whitelist
    let is_whitelisted = contract.whitelist(operator_address).call().await?;
    if !is_whitelisted {
        bail!("operator address {operator_address} is not whitelisted on the Lagrange contract. Contact Lagrange admin.");
    }
    let is_registered = contract.is_registered(operator_address).call().await?;
    if is_registered {
        bail!(
            "operator address {operator_address} is already registered on our contract! Exiting."
        );
    }

    println!(
        "Operator is whitelisted on Lagrange Network AVS contract. Moving on to registration."
    );

    let receipt = contract
        .register_operator(public_key, signature)
        .send()
        .await?
        .await?;

    println!(
        "Successfully registered on Lagrange AVS. Tx hash {:?}",
        receipt
            .expect("sucessful transaction but no receipt?")
            .transaction_hash
    );

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_network() {
        assert_eq!(
            Network::Holesky.lagrange_registry_address(),
            Network::Mainnet.lagrange_registry_address()
        );
        assert_eq!(
            Network::Holesky.lagrange_service_manager_address(),
            Network::Mainnet.lagrange_service_manager_address()
        );
        assert_eq!(
            Network::Holesky.eigen_avs_directory(),
            HOLESKY_AVS_DIRECTORY_ADDR.to_string().parse().unwrap()
        );
        assert_eq!(
            Network::Mainnet.eigen_avs_directory(),
            MAINNET_AVS_DIRECTORY_ADDR.to_string().parse().unwrap()
        );
        assert_eq!(
            Network::Holesky.eigen_delegation_manager_address(),
            HOLESKY_DELEGATION_MANAGER_ADDR.to_string().parse().unwrap()
        );
        assert_eq!(
            Network::Mainnet.eigen_delegation_manager_address(),
            MAINNET_DELEGATION_MANAGER_ADDR.to_string().parse().unwrap()
        );
    }
}
