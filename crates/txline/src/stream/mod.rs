//! Streaming module boundaries.
//!
//! Planned streaming support is based on Server-Sent Events for odds and scores:
//!
//! - send both TxLINE credentials and `Accept: text/event-stream`;
//! - parse `id`, `event`, `data`, and `retry` fields;
//! - handle heartbeat comments and no-data periods without treating them as
//!   failures;
//! - reconnect after transport failures;
//! - resume with `Last-Event-ID` when an event ID has been observed;
//! - renew only the guest JWT on 401 and reconnect with the existing API token.
//!
//! An open SSE connection can be healthy even when no covered fixture is
//! currently producing updates. This module currently only declares submodules.

pub mod odds;
pub mod scores;
pub mod sse;
