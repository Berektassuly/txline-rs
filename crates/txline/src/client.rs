//! Top-level client scaffolding.

use crate::config::TxlineConfig;

/// Entry point for future TxLINE SDK operations.
#[derive(Debug, Clone)]
pub struct TxlineClient {
    config: TxlineConfig,
}

impl TxlineClient {
    /// Create a client scaffold from configuration.
    pub fn new(config: TxlineConfig) -> Self {
        Self { config }
    }

    /// Borrow the client configuration.
    pub fn config(&self) -> &TxlineConfig {
        &self.config
    }
}
