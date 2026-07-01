//! Authentication lifecycle scaffolding.
//!
//! TODO: activation preimage is `${txSig}:${selectedLeagues.join(",")}:${jwt}`;
//! for empty leagues it is `${txSig}::${jwt}`.
//! TODO: renew the guest JWT on HTTP 401 while preserving the activated API
//! token for subsequent retries.

/// Guest JWT and activated API token placeholders.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthTokens {
    /// Guest JWT from the matching TxLINE host.
    pub jwt: Option<String>,
    /// Activated API token for data endpoints.
    pub api_token: Option<String>,
}
