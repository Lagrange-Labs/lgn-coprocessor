use std::io::Write;

use anyhow::Context;
use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use serde::Serialize;
use tracing::warn;

pub mod v1;

/// Prover trait.
///
/// Implementors of this trait shall support some of the message types, for each
/// support message types it must generate a proof.
pub trait LgnProver {
    fn run(
        &self,
        envelope: MessageEnvelope,
    ) -> anyhow::Result<MessageReplyEnvelope>;
}

/// Given a serializable circuit input, write it to a temporary file and log the
/// file path.
pub(crate) fn write_to_tmp<S: Serialize>(s: &S) -> anyhow::Result<()> {
    let mut tmp = tempfile::NamedTempFile::new().context("failed to create a temporary file")?;
    warn!("circuit inputs written to `{}`", tmp.path().display());
    tmp.write_all(
        serde_json::to_string_pretty(s)
            .context("failed to serialize PIs to JSON")?
            .as_bytes(),
    )
    .context("failed to write serialized PIs to temporary file")?;

    Ok(())
}
