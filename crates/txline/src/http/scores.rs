//! Scores snapshot, update, historical, stream, and validation endpoint scaffolding.
//!
//! Planned coverage includes `/api/scores/snapshot/{fixtureId}`,
//! `/api/scores/updates/...`, `/api/scores/historical/{fixtureId}`,
//! `/api/scores/stream`, and `/api/scores/stat-validation`.
//!
//! Validation requests must use a real score record sequence observed from a
//! score endpoint or stream event. The SDK should not synthesize `seq` values.
