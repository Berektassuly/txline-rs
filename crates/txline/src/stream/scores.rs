//! Scores stream scaffolding.
//!
//! Future score stream events can provide the real `Seq`/`seq` values used by
//! `/api/scores/stat-validation`. Implementations should keep event IDs for
//! resume and should treat quiet live windows as normal stream behavior.
