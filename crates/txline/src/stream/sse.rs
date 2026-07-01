//! Server-Sent Events transport scaffolding.
//!
//! Planned support includes parsing `id`, `event`, `data`, and `retry` fields;
//! ignoring heartbeat comments; reconnecting after drops; preserving
//! `Last-Event-ID`; and renewing the guest JWT on 401 while keeping the same
//! activated API token.
