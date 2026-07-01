//! Validation module boundaries.
//!
//! Planned validation support covers TxLINE score proof payloads and strategy
//! helpers. `/api/scores/stat-validation` supports two mutually exclusive
//! request modes:
//!
//! - legacy mode: `statKey` with optional `statKey2`;
//! - V2 mode: comma-separated `statKeys`.
//!
//! A future SDK should keep timestamp, PDA, proof, and strategy data aligned:
//! derive `daily_scores_roots` from the timestamp returned in the proof summary,
//! use the same timestamp in the on-chain call, and map stat proofs to requested
//! stat keys without reordering.
//!
//! The `seq` query parameter must come from a real score record observed through
//! snapshots, updates, historical data, or the scores stream. It is not a
//! placeholder and should not be synthesized by the SDK.

pub mod legacy;
pub mod proof;
pub mod strategy;
pub mod v2;
