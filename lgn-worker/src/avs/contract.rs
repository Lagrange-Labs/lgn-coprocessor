use std::sync::Arc;

use alloy::primitives::address;
use alloy::primitives::Address;
use alloy::primitives::U256;
use alloy::providers::RootProvider;
use alloy::signers::local::PrivateKeySigner;
use anyhow::bail;
use anyhow::Result;
use delegation_manager::DelegationManager;
use serde::Serialize;
use tracing::info;
use zkmr_stake_registry::ZKMRStakeRegistry;

pub use super::public_key::PublicKey;

/// ZKMR service manager address as an argument (avs) to call the contract
/// function `calculateOperatorAVSRegistrationDigestHash`
const HOLESKY_ZKMR_SERVICE_MANAGER_ADDR: Address =
    address!("0xf98D5De1014110C65c51b85Ea55f73863215CC10");
const MAINNET_ZKMR_SERVICE_MANAGER_ADDR: Address =
    address!("0x22CAc0e6A1465F043428e8AeF737b3cb09D0eEDa");

/// ZKMRStakeRegistry contract address
/// <https://github.com/Lagrange-Labs/lpn-relayer/blob/feat/avs-relay/src/config/chain.ts#L57>
const HOLESKY_ZKMR_STAKE_REGISTRY_ADDR: Address =
    address!("0xf724cDC7C40fd6B59590C624E8F0E5E3843b4BE4");
const MAINNET_ZKMR_STAKE_REGISTRY_ADDR: Address =
    address!("0x8dcdCc50Cc00Fe898b037bF61cCf3bf9ba46f15C");

/// AVSDirectory contract address
/// from https://github.com/Layr-Labs/eigenlayer-contracts?tab=readme-ov-file#deployments
const MAINNET_AVS_DIRECTORY_ADDR: Address = address!("0x135dda560e946695d6f155dacafc6f1f25c1f5af");
const HOLESKY_AVS_DIRECTORY_ADDR: Address = address!("0x055733000064333CaDDbC92763c58BF0192fFeBf");

/// DelegationManager contract address
/// from https://github.com/Layr-Labs/eigenlayer-contracts?tab=readme-ov-file#deployments
const MAINNET_DELEGATION_MANAGER_ADDR: Address =
    address!("0x39053D51B77DC0d36036Fc1fCc8Cb819df8Ef37A");
const HOLESKY_DELEGATION_MANAGER_ADDR: Address =
    address!("0xA44151489861Fe9e3055d95adC98FbD462B948e7");

mod avs_directory {
    use alloy::sol;

    sol!(
        #[sol(rpc)]
        AVSDirectory,
        concat!(env!("CARGO_MANIFEST_DIR"), "/abis/AVSDirectoryABI.json")
    );
}

mod delegation_manager {
    use alloy::sol;

    sol!(
        #[sol(rpc)]
        DelegationManager,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/abis/DelegationManagerABI.json"
        )
    );
}
mod zkmr_stake_registry {
    use alloy::sol;

    sol!(
        #[sol(rpc)]
        ZKMRStakeRegistry,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/abis/ZKMRStakeRegistryABI.json"
        )
    );
}

#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    #[default]
    Mainnet,
    Holesky,
    Hoodi,
}

impl Network {
    pub fn describe(&self) -> String {
        match self {
            Network::Mainnet => "mainnet",
            Network::Holesky => "holesky",
            Network::Hoodi => "hoodi",
        }
        .to_string()
    }

    pub fn chain_id(&self) -> u64 {
        match self {
            Network::Mainnet => 1,
            Network::Holesky => 17000,
            Network::Hoodi => 560048,
        }
    }

    /// Returns the address of the lagrange registry necessary to register an operator
    /// on Lagrange Network AVS
    fn lagrange_registry_address(&self) -> Address {
        match self {
            Network::Mainnet => MAINNET_ZKMR_STAKE_REGISTRY_ADDR,
            Network::Holesky => HOLESKY_ZKMR_STAKE_REGISTRY_ADDR,
            Network::Hoodi => todo!(),
        }
        .to_string()
        .parse()
        .expect("invalid registry address")
    }

    /// Returns the address of the service manager contract. Necessary input to
    /// compute the right avs digest hash for the registration signature.
    fn lagrange_service_manager_address(&self) -> Address {
        match self {
            Network::Mainnet => MAINNET_ZKMR_SERVICE_MANAGER_ADDR,
            Network::Holesky => HOLESKY_ZKMR_SERVICE_MANAGER_ADDR,
            Network::Hoodi => todo!(),
        }
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
            Network::Hoodi => todo!(),
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
            Network::Hoodi => todo!(),
        }
        .parse()
        .expect("invalid contract avs directory address")
    }
}

/// Call DelegationManager contract function `isOperator`
pub async fn is_operator(
    network: &Network,
    provider: Arc<RootProvider>,
    operator: Address,
) -> Result<bool> {
    let contract_address: Address = network.eigen_delegation_manager_address();
    let contract = DelegationManager::new(contract_address, provider);

    Ok(contract.isOperator(operator).call().await?)
}

/// Call AVSDirectory contract function `calculateOperatorAVSRegistrationDigestHash`
pub async fn calculate_registration_digest_hash(
    network: &Network,
    provider: Arc<RootProvider>,
    operator: Address,
    salt: [u8; 32],
    expiry: U256,
) -> Result<[u8; 32]> {
    let avs: Address = network.lagrange_service_manager_address();

    let contract_address: Address = network.eigen_avs_directory();
    let contract = avs_directory::AVSDirectory::new(contract_address, provider);

    let digest_hash = contract
        .calculateOperatorAVSRegistrationDigestHash(operator, avs, salt.into(), expiry)
        .call()
        .await?;

    Ok(digest_hash.into())
}

/// Call ZKMRStakeRegistry contract function `registerOperator`
pub async fn register_operator(
    network: &Network,
    client: RootProvider,
    signer: PrivateKeySigner,
    public_key: PublicKey,
    salt: [u8; 32],
    expiry: U256,
    signature: Vec<u8>,
) -> Result<()> {
    let operator_address = signer.address();
    let contract_address: Address = network.lagrange_registry_address();
    let contract = ZKMRStakeRegistry::new(contract_address, client);

    let public_key = ZKMRStakeRegistry::PublicKey {
        x: public_key.x,
        y: public_key.y,
    };
    let signature = zkmr_stake_registry::ISignatureUtils::SignatureWithSaltAndExpiry {
        expiry,
        salt: salt.into(),
        signature: signature.into(),
    };
    // we first check if we are whitelist
    let is_whitelisted = contract.whitelist(operator_address).call().await?;
    if !is_whitelisted {
        bail!("operator address {operator_address} is not whitelisted on the Lagrange contract. Contact Lagrange admin.");
    }
    let is_registered = contract.isRegistered(operator_address).call().await?;
    if is_registered {
        bail!(
            "operator address {operator_address} is already registered on our contract! Exiting."
        );
    }

    info!("Operator is whitelisted on Lagrange Network AVS contract. Moving on to registration.");

    let transaction = contract
        .registerOperator(public_key, signature)
        .send()
        .await?;

    info!(
        "Successfully registered on Lagrange AVS. Tx hash {:?}",
        transaction.tx_hash()
    );

    Ok(())
}

/// Call ZKMRStakeRegistry contract function `evictOperator`
pub async fn deregister_operator(
    network: &Network,
    client: &RootProvider,
) -> Result<()> {
    let contract_address: Address = network.lagrange_registry_address();
    let contract = ZKMRStakeRegistry::new(contract_address, client);
    let receipt = contract
        .deregisterOperator()
        .send()
        .await?
        .get_receipt()
        .await?;

    info!(
        "Successfully de-registered from Lagrange AVS. Tx hash {:?}",
        receipt.transaction_hash
    );

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_network() {
        assert_eq!(
            Network::Holesky.eigen_avs_directory(),
            HOLESKY_AVS_DIRECTORY_ADDR
        );
        assert_eq!(
            Network::Mainnet.eigen_avs_directory(),
            MAINNET_AVS_DIRECTORY_ADDR
        );
        assert_eq!(
            Network::Holesky.eigen_delegation_manager_address(),
            HOLESKY_DELEGATION_MANAGER_ADDR
        );
        assert_eq!(
            Network::Mainnet.eigen_delegation_manager_address(),
            MAINNET_DELEGATION_MANAGER_ADDR
        );
    }
}
