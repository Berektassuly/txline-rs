//! Devnet configuration.

use crate::{Result, TxlineError};

pub const DEVNET_API_HOST: &str = "https://txline-dev.txodds.com";
pub const DEVNET_API_BASE: &str = "https://txline-dev.txodds.com/api";
pub const DEVNET_GUEST_AUTH_URL: &str = "https://txline-dev.txodds.com/auth/guest/start";
pub const DEVNET_PROGRAM_ID: &str = "6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J";
pub const DEVNET_TXL_MINT: &str = "4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG";
pub const DEVNET_USDT_MINT: &str = "ELWTKspHKCnCfCiCiqYw1EDH77k8VCP74dK9qytG2Ujh";
pub const DEVNET_RPC_URL: &str = "https://api.devnet.solana.com";

/// TxLINE deployment target.
///
/// Only Devnet is supported in this crate version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    Devnet,
}

/// SDK configuration for the TxLINE Devnet deployment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxlineConfig {
    pub network: Network,
    pub api_host: String,
    pub api_base: String,
    pub guest_auth_url: String,
    pub program_id: String,
    pub txl_mint: String,
    pub usdt_mint: String,
    pub rpc_url: String,
}

impl TxlineConfig {
    /// Build the canonical Devnet configuration.
    pub fn devnet() -> Self {
        Self {
            network: Network::Devnet,
            api_host: DEVNET_API_HOST.to_owned(),
            api_base: DEVNET_API_BASE.to_owned(),
            guest_auth_url: DEVNET_GUEST_AUTH_URL.to_owned(),
            program_id: DEVNET_PROGRAM_ID.to_owned(),
            txl_mint: DEVNET_TXL_MINT.to_owned(),
            usdt_mint: DEVNET_USDT_MINT.to_owned(),
            rpc_url: DEVNET_RPC_URL.to_owned(),
        }
    }

    /// Override the Solana RPC URL while keeping all TxLINE Devnet values fixed.
    ///
    /// Callers must provide a Devnet RPC endpoint. Validation rejects obvious
    /// mainnet-looking URLs, but custom providers cannot be fully verified
    /// syntactically.
    pub fn with_rpc_url(mut self, rpc_url: impl Into<String>) -> Self {
        self.rpc_url = rpc_url.into();
        self
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.network != Network::Devnet {
            return Err(TxlineError::config(
                "only TxLINE Devnet is supported by this SDK build",
            ));
        }
        if self.api_host != DEVNET_API_HOST
            || self.api_base != DEVNET_API_BASE
            || self.guest_auth_url != DEVNET_GUEST_AUTH_URL
            || self.program_id != DEVNET_PROGRAM_ID
            || self.txl_mint != DEVNET_TXL_MINT
            || self.usdt_mint != DEVNET_USDT_MINT
        {
            return Err(TxlineError::config(
                "TxLINE Devnet config values must not be mixed with other networks",
            ));
        }
        if self.rpc_url.trim().is_empty() {
            return Err(TxlineError::config("Solana RPC URL must not be empty"));
        }
        if looks_like_mainnet_rpc_url(&self.rpc_url) {
            return Err(TxlineError::config(
                "Solana RPC URL must be a Devnet RPC endpoint for this SDK build",
            ));
        }
        Ok(())
    }
}

impl Default for TxlineConfig {
    fn default() -> Self {
        Self::devnet()
    }
}

fn looks_like_mainnet_rpc_url(rpc_url: &str) -> bool {
    let lower = rpc_url.trim().to_ascii_lowercase();
    lower
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|part| part == "mainnet" || part == "mainnetbeta")
}
