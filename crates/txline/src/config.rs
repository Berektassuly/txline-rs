//! Network configuration scaffolding.

/// TxLINE deployment target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Network {
    /// TxLINE devnet deployment.
    Devnet,
    /// TxLINE mainnet deployment.
    Mainnet,
    /// Caller-supplied deployment values.
    Custom(String),
}

/// SDK configuration placeholder.
///
/// TODO: prevent mixing Devnet/Mainnet RPC, API host, program ID, and token
/// mints when concrete fields are added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxlineConfig {
    /// Selected deployment target.
    pub network: Network,
}

impl TxlineConfig {
    /// Create a config scaffold for a network.
    pub fn new(network: Network) -> Self {
        Self { network }
    }
}
