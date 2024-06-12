use config::FileFormat;
use serde_derive::Deserialize;

use lazy_static_include::*;
use lgn_messages::types::WorkerClass;
use redact::Secret;
use tracing::debug;

lazy_static_include_str! {
    DEFAULT_CONFIG => "src/config/default.toml",
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Config {
    pub(crate) worker: WorkerConfig,
    pub(crate) avs: AvsConfig,
    pub(crate) public_params: PublicParamsConfig,
    pub(crate) prometheus: PrometheusConfig,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct PublicParamsConfig {
    pub(crate) url: String,
    pub(crate) dir: String,
    pub(crate) checksum: String,
    /// If set to true, the parameters will not be written to disk, ever.
    pub(crate) skip_store: bool,
    pub(crate) preprocessing_params: PreprocessingParams,
    pub(crate) query2_params: Query2Params,
    pub(crate) groth16_assets: Groth16Assets,
}

impl PublicParamsConfig {
    pub fn validate(&self) {
        assert!(!self.url.is_empty(), "URL is required");
        assert!(!self.dir.is_empty(), "Directory is required");
        self.preprocessing_params.validate();
        self.query2_params.validate();
        self.groth16_assets.validate();
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct PreprocessingParams {
    pub(crate) file: String,
}

impl PreprocessingParams {
    pub fn validate(&self) {
        assert!(!self.file.is_empty(), "Preprocessing file is required");
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Query2Params {
    pub(crate) file: String,
}

impl Query2Params {
    pub fn validate(&self) {
        assert!(!self.file.is_empty(), "Query2 file is required");
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Groth16Assets {
    pub(crate) circuit_file: String,
    pub(crate) circuit_file_checksum: String,
    pub(crate) r1cs_file: String,
    pub(crate) r1cs_file_checksum: String,
    pub(crate) pk_file: String,
    pub(crate) pk_file_checksum: String,
}

impl Groth16Assets {
    pub fn validate(&self) {
        assert!(!self.circuit_file.is_empty(), "Circuit URL is required");
        assert!(!self.r1cs_file.is_empty(), "R1CS URL is required");
        assert!(!self.pk_file.is_empty(), "PK URL is required");
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct WorkerConfig {
    pub(crate) instance_type: WorkerClass,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct AvsConfig {
    pub(crate) gateway_url: String,
    pub(crate) issuer: String,
    pub(crate) worker_id: String,
    pub(crate) lagr_keystore: Option<String>,
    pub(crate) lagr_pwd: Option<Secret<String>>,
    pub(crate) lagr_private_key: Option<Secret<String>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct PrometheusConfig {
    pub(crate) port: u16,
}

impl AvsConfig {
    pub fn validate(&self) {
        assert!(!self.gateway_url.is_empty(), "Gateway URL is required");
        assert!(!self.issuer.is_empty(), "Issuer is required");
        assert!(!self.worker_id.is_empty(), "Worker ID is required");

        match (&self.lagr_keystore, &self.lagr_pwd, &self.lagr_private_key) {
            (Some(kpath), Some(pwd), _) => {
                assert!(!kpath.is_empty(), "Keystore path is empty");
                assert!(!pwd.expose_secret().is_empty(), "Password is empty");
            }
            (None, None, Some(pkey)) => assert!(
                !pkey.expose_secret().is_empty(),
                "Private key value is empty"
            ),
            _ => (),
        }
    }
}

impl Config {
    pub fn load(local_file: Option<String>) -> Config {
        let mut config_builder = config::Config::builder();
        config_builder =
            config_builder.add_source(config::File::from_str(&DEFAULT_CONFIG, FileFormat::Toml));

        if let Some(local_file) = local_file {
            debug!("Loading local configuration from {}", local_file);
            config_builder = config_builder.add_source(config::File::with_name(&local_file));
        }

        let config_builder = config_builder
            .add_source(
                config::Environment::default()
                    .separator("__")
                    .ignore_empty(true),
            )
            .build()
            .expect("Could not load configuration");

        config_builder
            .try_deserialize()
            .expect("Could not deserialize configuration")
    }

    pub fn validate(&self) {
        self.public_params.validate();
        self.avs.validate();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use lgn_provers::provers::v0::query::prover::QueryStorageProver;
    use std::path::Path;

    #[test]
    #[ignore]
    fn test_skip_storing_params() {
        let mut config_builder = config::Config::builder();
        config_builder =
            config_builder.add_source(config::File::from_str(&DEFAULT_CONFIG, FileFormat::Toml));
        let config_builder = config_builder
            .add_source(
                config::Environment::default()
                    .separator("__")
                    .ignore_empty(true),
            )
            .build()
            .expect("Could not load configuration");

        let mut conf: Config = config_builder
            .try_deserialize()
            .expect("Could not deserialize configuration");
        conf.public_params.skip_store = true;
        QueryStorageProver::init(
            &conf.public_params.url,
            &conf.public_params.dir.clone(),
            &conf.public_params.query2_params.file.clone(),
            &conf.public_params.checksum,
            conf.public_params.skip_store,
        )
        .expect("this should work");
        // test if the file exists, it should not
        let path =
            Path::new(&conf.public_params.dir).join(conf.public_params.query2_params.file.clone());
        // delete it in case it already exists
        std::fs::remove_file(&path).expect("should delete");
        assert!(!path.exists(), "query param file should not exist");
        conf.public_params.skip_store = false;
        QueryStorageProver::init(
            &conf.public_params.url,
            &conf.public_params.dir.clone(),
            &conf.public_params.query2_params.file.clone(),
            &conf.public_params.checksum,
            conf.public_params.skip_store,
        )
        .expect("this should work");
        // test if the file exists, it should exist by now
        assert!(path.exists(), "query param file should exist");
        std::fs::remove_file(path).expect("should delete");
    }
}
