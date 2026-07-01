//! V2 score validation scaffolding.
//!
//! V2 score validation uses `/api/scores/stat-validation` with a comma-separated
//! `statKeys` query value and returns `ScoresStatValidationV2`. The order of
//! the requested stat keys is significant: `statsToProve[index]` pairs with
//! `statProofs[index]`, and strategy indices refer to those positions.
//!
//! For example, `statKeys=1,2,3001,3002` means strategy index `2` refers to
//! stat key `3001`, not to the numeric key `2`.
//!
//! Per the CTO update captured for this scaffold on 2026-07-01, Mainnet and
//! Devnet are equivalent for the newest V2 score-validation flow. The new
//! Mainnet score proof behavior applies to score records from
//! `2026-07-01 08:00 GMT` onward. Older score records may still need legacy
//! proof handling.
