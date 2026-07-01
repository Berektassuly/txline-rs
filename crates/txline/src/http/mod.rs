//! HTTP data access module boundaries.
//!
//! Planned REST coverage follows the hosted OpenAPI:
//!
//! - fixtures: snapshots, updates, fixture validation, and batch validation;
//! - odds: snapshots, updates, streams, and validation;
//! - scores: snapshots, updates, historical data, streams, and
//!   `/api/scores/stat-validation`.
//!
//! Most data endpoints require both the guest JWT in `Authorization` and the
//! activated API token in `X-Api-Token`. HTTP implementations should redact both
//! credentials in logs and should treat 401 as guest-JWT renewal, while treating
//! 403 as an entitlement, expiry, invalid token, or network-mismatch signal.
//!
//! This module currently only declares submodules.

pub mod fixtures;
pub mod models;
pub mod odds;
pub mod scores;
