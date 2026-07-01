//! Validation strategy builder scaffolding.
//!
//! Future strategy builders should model the N-dimensional strategy shape used
//! by the TxLINE program:
//!
//! - single-stat predicates by positional index;
//! - binary expressions over two positional indices;
//! - geometric targets and distance predicates;
//! - multi-leg strategies that combine several predicates.
//!
//! Strategy indices are positions in the V2 `statKeys` request order, not raw
//! stat key values. Builders should keep the requested stat keys visible so a
//! caller can audit what each index means before submitting an on-chain
//! validation call.
