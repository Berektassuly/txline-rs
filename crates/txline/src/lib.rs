//! Devnet-only Rust SDK for TxLINE.
//!
//! This crate intentionally supports TxLINE Devnet only in this implementation
//! phase. Mainnet constants, examples, feature flags, and transaction flows are
//! out of scope until the Devnet path has been exercised end to end.

pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod http;
pub mod solana;
pub mod stream;
pub mod validation;

pub use auth::{ApiToken, AuthHeaders, GuestJwt, GuestSession, activation_preimage};
pub use client::TxlineClient;
pub use config::{DEVNET_API_BASE, DEVNET_API_HOST, DEVNET_GUEST_AUTH_URL, Network, TxlineConfig};
pub use error::{Result, TxlineError};
pub use solana::transaction_safety::ValidatedPurchaseQuote;
