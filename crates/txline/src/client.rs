//! Top-level client scaffolding.
//!
//! [`TxlineClient`] is the future facade for the SDK. It currently stores only a
//! [`TxlineConfig`](crate::config::TxlineConfig); no transport, auth state,
//! Solana client, stream client, or validation engine is wired in yet.
//!
//! Future client methods should make scaffold status explicit until their
//! modules contain real behavior. For example, a method that fetches scores
//! should not appear in public docs until it actually sends authenticated HTTP
//! requests and maps the OpenAPI response shape.

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
