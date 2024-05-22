//! Message routing high level semantic.
use serde_derive::{Deserialize, Serialize};

/// Max number of priorities we support globally
pub const PRIORITIES_LIMIT: u8 = 100;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum RoutingKey {
    /// Using integers for priority, for example: p1, p2, p3... p10
    /// WARNING: Check [`PRIORITIES_LIMIT`]
    Priority(u8),

    /// Can specify domain, something like "sq" (storage query)
    Domain(String),

    /// Can specify both domain and priority
    /// WARNING: Check [`PRIORITIES_LIMIT`]
    Combined(String, u8),
}

impl RoutingKey {
    pub fn priority(p: u8) -> anyhow::Result<Self> {
        if p > PRIORITIES_LIMIT {
            anyhow::bail!(
                "Priority value {} is greater than the limit {}",
                p,
                PRIORITIES_LIMIT
            );
        }
        Ok(RoutingKey::Priority(p))
    }

    pub fn domain(domain: String) -> anyhow::Result<Self> {
        Ok(RoutingKey::Domain(domain))
    }

    pub fn combined(domain: String, priority: u8) -> anyhow::Result<Self> {
        if priority > PRIORITIES_LIMIT {
            anyhow::bail!(
                "Priority value {} is greater than the limit {}",
                priority,
                PRIORITIES_LIMIT
            );
        }
        Ok(RoutingKey::Combined(domain, priority))
    }

    /// Get the route string for this routing key.
    pub fn get_route(&self) -> anyhow::Result<String> {
        match self {
            RoutingKey::Priority(p) => Ok(format!("p{p}")),
            RoutingKey::Domain(d) => Ok(d.clone()),
            RoutingKey::Combined(domain, priority) => Ok(format!("{domain}_p{priority:?}")),
        }
    }
}
