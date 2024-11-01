//! Message routing high level semantic.
use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct RoutingKey
{
    domain: String,
    priority: u64,
}

impl RoutingKey
{
    pub fn combined(
        domain: String,
        priority: u64,
    ) -> Self
    {
        RoutingKey {
            domain,
            priority,
        }
    }

    pub fn priority(&self) -> u64
    {
        self.priority
    }

    /// Get the route string for this routing key.
    pub fn get_route(&self) -> String
    {
        self.domain
            .clone()
    }
}
