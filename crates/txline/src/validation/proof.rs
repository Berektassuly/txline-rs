//! Proof decoding and representation scaffolding.
//!
//! Future proof support should decode and validate the shapes returned by the
//! hosted OpenAPI before they are passed to Solana/Anchor bindings. Proof hashes
//! must decode to exactly 32 bytes, and sibling direction must be preserved.
//!
//! The SDK should support both legacy score proof data and the newer compact
//! score proof trees used by Mainnet score records from
//! `2026-07-01 08:00 GMT` onward. Backward compatibility matters because users
//! may validate historical records after the new proof behavior is live.
//!
//! Proof objects should keep enough non-secret context for diagnostics: network,
//! fixture ID, sequence, timestamp, derived epoch day, and PDA.
