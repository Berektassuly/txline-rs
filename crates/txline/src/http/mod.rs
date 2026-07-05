//! REST clients for the published TxLINE Devnet API.

pub mod fixtures;
pub mod models;
pub mod odds;
pub mod scores;

pub use models::{
    BatchMetadata, Fixture, FixtureBatchSummary, FixtureBatchValidation, FixtureValidation,
    OddsBatchSummary, OddsPayload, OddsValidation, PlayerStats, PlayerStatsForParticipants,
    PurchaseQuoteRequest, PurchaseQuoteResponse, Scores, UpdateStats,
};
