//! Message routing high level semantic.
use serde_derive::Deserialize;
use serde_derive::Serialize;

/// The routing domain for a message.
///
/// NOTE: This is no longer used, maintained for backwards compatibility.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct RoutingKey {
    domain: String,
    priority: u64,
}

impl RoutingKey {
    /// Creates a new [RoutingKey] given the `domain` and `priority`.
    pub fn combined(
        domain: String,
        priority: u64,
    ) -> Self {
        RoutingKey { domain, priority }
    }
}
