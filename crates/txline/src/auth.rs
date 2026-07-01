//! Authentication lifecycle scaffolding.
//!
//! TxLINE currently uses two credentials with different lifecycles:
//!
//! - A guest JWT from `/auth/guest/start`, sent as
//!   `Authorization: Bearer <jwt>`.
//! - An activated API token from `/api/token/activate`, sent as
//!   `X-Api-Token: <api-token>`.
//!
//! The OpenAPI description states that the guest JWT expires after 30 days.
//! Future retry logic should renew the guest JWT on HTTP 401 from the same host
//! and preserve the activated API token for the retried request. Re-activation
//! is not required just because the guest JWT expired.
//!
//! Activation signs the strict preimage
//! `${txSig}:${selectedLeagues.join(",")}:${jwt}`. For an empty league list the
//! signed message is `${txSig}::${jwt}`. The detached signature is base64
//! encoded and posted with the confirmed `txSig` and `leagues` array.
//!
//! Never log guest JWTs, activated API tokens, private keys, or unredacted
//! request headers.

/// Guest JWT and activated API token placeholders.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthTokens {
    /// Guest JWT from the matching TxLINE host.
    pub jwt: Option<String>,
    /// Activated API token for data endpoints.
    pub api_token: Option<String>,
}
