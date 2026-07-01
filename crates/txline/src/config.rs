//! Network configuration scaffolding.
//!
//! Future configuration should bind each TxLINE network to one consistent set
//! of values:
//!
//! - Solana RPC URL.
//! - API base URL.
//! - guest JWT URL.
//! - activation URL.
//! - TxLINE program ID.
//! - TxL Token-2022 mint.
//!
//! The verified hosts and addresses at the time of this scaffold are:
//!
//! - Mainnet API: `https://txline.txodds.com/api/`.
//! - Mainnet guest JWT: `https://txline.txodds.com/auth/guest/start`.
//! - Mainnet program: `9ExbZjAapQww1vfcisDmrngPinHTEfpjYRWMunJgcKaA`.
//! - Mainnet TxL mint: `Zhw9TVKp68a1QrftncMSd6ELXKDtpVMNuMGr1jNwdeL`.
//! - Devnet API: `https://txline-dev.txodds.com/api/`.
//! - Devnet guest JWT: `https://txline-dev.txodds.com/auth/guest/start`.
//! - Devnet program: `6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J`.
//! - Devnet TxL mint: `4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG`.
//!
//! Configuration code should reject accidental Mainnet/Devnet mixing before any
//! request, transaction, or validation proof is attempted.

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
