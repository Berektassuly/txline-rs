//! Rust SDK scaffold for TxLINE.
//!
//! This crate is intentionally pre-implementation. It records the intended
//! public module layout for a future TxLINE Rust SDK, but it does not yet make
//! HTTP requests, open SSE streams, sign Solana transactions, activate API
//! tokens, decode Merkle proofs, or call on-chain validation instructions.
//!
//! Planned areas:
//!
//! - [`config`] keeps network-specific hosts, program IDs, mint addresses, and
//!   guardrails against mixing Mainnet and Devnet values.
//! - [`auth`] manages the guest JWT and activated API token lifecycle.
//! - [`client`] will become the high-level SDK entry point.
//! - [`http`] will cover fixtures, odds, scores, and proof endpoints when the
//!   `http` feature is enabled.
//! - [`stream`] will cover Server-Sent Events when the `stream` feature is
//!   enabled.
//! - [`solana`] will cover Token-2022 accounts, PDAs, subscriptions, purchase
//!   transactions, and transaction safety checks when the `solana` feature is
//!   enabled.
//! - [`validation`] will cover legacy and V2 score validation when the
//!   `validation` feature is enabled.
//!
//! See the repository README and `docs/` directory for the researched TxLINE
//! integration notes that should guide future implementation work.

pub mod auth;
pub mod client;
pub mod config;
pub mod error;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "solana")]
pub mod solana;

#[cfg(feature = "stream")]
pub mod stream;

#[cfg(feature = "validation")]
pub mod validation;

pub use client::TxlineClient;
pub use config::{Network, TxlineConfig};
pub use error::{Result, TxlineError};
