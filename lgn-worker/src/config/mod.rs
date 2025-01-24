use config::FileFormat;
use lazy_static_include::*;
use lgn_messages::types::TaskDifficulty;
use lgn_provers::params::PARAMS_CHECKSUM_FILENAME;
use redact::Secret;
use serde_derive::Deserialize;
use tracing::debug;

lazy_static_include_str! {
    DEFAULT_CONFIG => "src/config/default.toml",
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Config
{
    pub(crate) worker: WorkerConfig,
    pub(crate) avs: AvsConfig,
    pub(crate) public_params: PublicParamsConfig,
    pub(crate) prometheus: PrometheusConfig,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct PublicParamsConfig
{
    /// the root URL over which we should fetch params.
    /// The FULL url is constructed from this one and the mp2 version
    pub(crate) params_root_url: String,
    pub(crate) checksum_expected_local_path: String,
    pub(crate) skip_checksum: bool,
    pub(crate) dir: String,
    /// If set to true, the parameters will not be written to disk, ever.
    pub(crate) skip_store: bool,
    pub(crate) preprocessing_params: PreprocessingParams,
    pub(crate) query_params: QueryParams,
    pub(crate) groth16_assets: Groth16Assets,
}

impl PublicParamsConfig
{
    pub fn validate(&self)
    {
        assert!(
            !self
                .params_root_url
                .is_empty(),
            "URL is required"
        );
        assert!(
            !self
                .checksum_expected_local_path
                .is_empty(),
            "Checksum local path for expected checksum file is required"
        );
        assert!(
            !self
                .dir
                .is_empty(),
            "Directory is required"
        );
        self.preprocessing_params
            .validate();
        self.query_params
            .validate();
        self.groth16_assets
            .validate();
    }

    /// Build the base URL with path of mp2 version for downloading param files.
    pub fn params_base_url(&self) -> String
    {
        add_mp2_version_path_to_url(&self.params_root_url)
    }

    /// Build the URL for downloading the checksum file.
    pub fn checksum_file_url(&self) -> String
    {
        let url = self.params_base_url();
        format!("{url}/{PARAMS_CHECKSUM_FILENAME}")
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct PreprocessingParams
{
    pub(crate) file: String,
}

impl PreprocessingParams
{
    pub fn validate(&self)
    {
        assert!(
            !self
                .file
                .is_empty(),
            "Preprocessing file is required"
        );
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct QueryParams
{
    pub(crate) file: String,
}

impl QueryParams
{
    pub fn validate(&self)
    {
        assert!(
            !self
                .file
                .is_empty(),
            "Query2 file is required"
        );
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Groth16Assets
{
    pub(crate) circuit_file: String,
    pub(crate) r1cs_file: String,
    pub(crate) pk_file: String,
}

impl Groth16Assets
{
    pub fn validate(&self)
    {
        assert!(
            !self
                .circuit_file
                .is_empty(),
            "Circuit URL is required"
        );
        assert!(
            !self
                .r1cs_file
                .is_empty(),
            "R1CS URL is required"
        );
        assert!(
            !self
                .pk_file
                .is_empty(),
            "PK URL is required"
        );
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct WorkerConfig
{
    pub(crate) instance_type: TaskDifficulty,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct AvsConfig
{
    pub(crate) gateway_url: String,
    pub(crate) gateway_grpc_url: Option<String>,
    pub(crate) max_grpc_message_size_mb: Option<usize>,
    pub(crate) issuer: String,
    pub(crate) worker_id: String,
    pub(crate) lagr_keystore: Option<String>,
    pub(crate) lagr_pwd: Option<Secret<String>>,
    pub(crate) lagr_private_key: Option<Secret<String>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct PrometheusConfig
{
    pub(crate) port: u16,
}

impl AvsConfig
{
    pub fn validate(&self)
    {
        assert!(
            !self
                .gateway_url
                .is_empty(),
            "Gateway URL is required"
        );
        assert!(
            !self
                .issuer
                .is_empty(),
            "Issuer is required"
        );
        assert!(
            !self
                .worker_id
                .is_empty(),
            "Worker ID is required"
        );

        match (
            &self.lagr_keystore,
            &self.lagr_pwd,
            &self.lagr_private_key,
        )
        {
            (Some(kpath), Some(pwd), _) =>
            {
                assert!(
                    !kpath.is_empty(),
                    "Keystore path is empty"
                );
                assert!(
                    !pwd.expose_secret()
                        .is_empty(),
                    "Password is empty"
                );
            },
            (None, None, Some(pkey)) =>
            {
                assert!(
                    !pkey
                        .expose_secret()
                        .is_empty(),
                    "Private key value is empty"
                )
            },
            _ => (),
        }
    }
}

impl Config
{
    pub fn load(local_file: Option<String>) -> Config
    {
        let mut config_builder = config::Config::builder();
        config_builder = config_builder.add_source(
            config::File::from_str(
                &DEFAULT_CONFIG,
                FileFormat::Toml,
            ),
        );

        if let Some(local_file) = local_file
        {
            debug!(
                "Loading local configuration from {}",
                local_file
            );
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

    pub fn validate(&self)
    {
        self.public_params
            .validate();
        self.avs
            .validate();
    }
}

/// Add mp2 version as a path to the base URL.
/// e.g. https://base.com/MP2_VERSION
fn add_mp2_version_path_to_url(url: &str) -> String
{
    let mp2_ver = mp2_common::git::short_git_version();
    format!("{url}/{mp2_ver}")
}
