//! Legacy score validation scaffolding.
//!
//! Legacy score validation uses `/api/scores/stat-validation` with `statKey`
//! and optional `statKey2`. The response shape is `ScoresStatValidation`:
//! `statToProve`/`statProof` for the first stat and optional
//! `statToProve2`/`statProof2` for the second stat.
//!
//! Future code should retain this compatibility for older score records and for
//! callers that only need one-stat or two-stat predicates. It should also make
//! timestamp and PDA alignment explicit, because proof payloads are only valid
//! against the matching `daily_scores_roots` account.
