use anyhow::*;
use checksum::{fetch_checksum_file, verify_directory_checksums};
use clap::Parser;
use lgn_messages::types::{DownstreamPayload, ReplyType, TaskType};
use manager::{v1::register_v1_provers, ProversManager};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

mod checksum;
mod config;
mod manager;

#[derive(Parser, Clone, Debug)]
/// Run the prover against a JSON file containing a task envelope as sent by the
/// QE.
struct Cli {
    #[clap(short, long)]
    /// The config file; `$(toml-worker-lgn)` can be used if devenv is enabled.
    config: String,

    #[clap()]
    /// The proof public inputs
    input: String,
}

fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .pretty()
        .compact()
        .with_level(true)
        .with_file(false)
        .with_line_number(false)
        .without_time()
        .with_target(false)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting up logging failed");

    let cli = Cli::parse();

    let config = config::Config::load(Some(cli.config));
    config.validate();

    let checksum_url = &config.public_params.checksum_url;
    let expected_checksums_file = &config.public_params.checksum_expected_local_path;
    info!("Fetching the checksum file... ");
    fetch_checksum_file(checksum_url, expected_checksums_file)?;
    info!("done.");

    info!("Initializing the provers... ");
    let mut provers_manager = ProversManager::<TaskType, ReplyType>::new();
    info!("done.");

    info!("Registering the provers... ");
    register_v1_provers(&config, &mut provers_manager).context("while registering provers")?;
    info!("done.");

    if !config.public_params.skip_checksum {
        verify_directory_checksums(&config.public_params.dir, expected_checksums_file)
            .context("Failed to verify checksums")?;
    }

    let content = std::fs::read_to_string(&cli.input)
        .with_context(|| format!("failed to open `{}`", cli.input))?;

    match serde_json::from_str::<DownstreamPayload<TaskType>>(&content)? {
        DownstreamPayload::Todo { envelope } => {
            provers_manager.delegate_proving(&envelope)?;
        }
        DownstreamPayload::Ack => bail!("unexpected ACK frame"),
    };

    Ok(())
}
