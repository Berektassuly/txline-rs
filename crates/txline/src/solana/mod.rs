//! Solana helper module boundaries.
//!
//! Planned Solana support covers the on-chain side of TxLINE access:
//!
//! - Token-2022 associated token accounts for the user and treasury vaults;
//! - subscription transactions through `subscribe(serviceLevelId, weeks)`;
//! - optional purchase quote transactions for paid TxL flows;
//! - Program Derived Addresses (PDAs) such as `pricing_matrix`,
//!   `token_treasury_v2`, `daily_scores_roots`, `daily_batch_roots`, and
//!   `ten_daily_fixtures_roots`;
//! - transaction safety checks before any SDK-assisted signing.
//!
//! The SDK should verify purchase quote transactions locally before signing:
//! expected fee payer, backend/admin signature where required, allowed program
//! IDs, expected instruction name, requested amount, and instruction count.
//!
//! Free tiers do not require TxL payment, but they still require SOL for normal
//! Solana fees and possible account rent. This module currently only declares
//! submodules.

pub mod pda;
pub mod purchase;
pub mod subscription;
pub mod transaction_safety;
