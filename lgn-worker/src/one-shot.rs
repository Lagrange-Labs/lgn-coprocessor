#![feature(generic_const_exprs)]
use anyhow::*;
use checksum::fetch_checksums;
use clap::Parser;
use lgn_messages::types::MessageEnvelope;
use manager::ProversManager;
use tracing::error;
use tracing::level_filters::LevelFilter;
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

#[tokio::main]
async fn main() -> Result<()> {
    std::panic::set_hook(Box::new(|panic_info| {
        let msg = match panic_info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => {
                match panic_info.payload().downcast_ref::<String>() {
                    Some(s) => &s[..],
                    None => "Box<dyn Any>",
                }
            },
        };
        let (file, lineno, col) = match panic_info.location() {
            Some(l) => (l.file(), l.line(), l.column()),
            None => ("<unknown>", 0, 0),
        };

        error!(
            msg,
            file,
            lineno,
            col,
            "Panic occurred: {:?}",
            backtrace::Backtrace::new(),
        );
    }));

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
    let checksums = fetch_checksums(config.public_params.checksum_file_url()).await?;

    let provers_manager =
        ProversManager::new(&config, &checksums).context("while registering provers")?;

    let envelope = std::fs::read_to_string(&cli.input)
        .with_context(|| format!("failed to open `{}`", cli.input))
        .and_then(|content| {
            serde_json::from_str::<MessageEnvelope>(&content).context("failed to parse input JSON")
        })?;

    provers_manager
        .delegate_proving(envelope)
        .context("proof failed")?;

    Ok(())
}
