//! Rust SDK scaffold for TxLINE.
//!
//! This crate currently contains module boundaries only. Future work can fill
//! each area without changing the public workspace shape.

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
