//! SDK error types.

use std::fmt;

use thiserror::Error;

/// SDK result type.
pub type Result<T> = std::result::Result<T, TxlineError>;

#[derive(Error)]
pub enum TxlineError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("missing guest JWT; call start_guest_session or set_guest_jwt first")]
    MissingGuestJwt,

    #[error("missing API token; activate a subscription or call set_api_token first")]
    MissingApiToken,

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("proof decode error: {0}")]
    ProofDecode(String),

    #[error("validation payload error: {0}")]
    Validation(String),

    #[error("HTTP {status}: {}", sanitized_http_status_body(body))]
    HttpStatus { status: u16, body: String },

    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("invalid HTTP header value: {0}")]
    Header(#[from] reqwest::header::InvalidHeaderValue),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Solana error: {0}")]
    Solana(String),

    #[error("Solana RPC error: {0}")]
    SolanaRpc(#[from] solana_client::client_error::ClientError),
}

impl fmt::Debug for TxlineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(message) => f.debug_tuple("Config").field(message).finish(),
            Self::MissingGuestJwt => f.write_str("MissingGuestJwt"),
            Self::MissingApiToken => f.write_str("MissingApiToken"),
            Self::InvalidInput(message) => f.debug_tuple("InvalidInput").field(message).finish(),
            Self::ProofDecode(message) => f.debug_tuple("ProofDecode").field(message).finish(),
            Self::Validation(message) => f.debug_tuple("Validation").field(message).finish(),
            Self::HttpStatus { status, body } => f
                .debug_struct("HttpStatus")
                .field("status", status)
                .field("body", &sanitized_http_status_body(body))
                .finish(),
            Self::Http(err) => f.debug_tuple("Http").field(err).finish(),
            Self::Url(err) => f.debug_tuple("Url").field(err).finish(),
            Self::Header(err) => f.debug_tuple("Header").field(err).finish(),
            Self::Json(err) => f.debug_tuple("Json").field(err).finish(),
            Self::Base64(err) => f.debug_tuple("Base64").field(err).finish(),
            Self::Solana(message) => f.debug_tuple("Solana").field(message).finish(),
            Self::SolanaRpc(err) => f.debug_tuple("SolanaRpc").field(err).finish(),
        }
    }
}

fn sanitized_http_status_body(body: &str) -> String {
    if body.is_empty() {
        "response body empty".to_owned()
    } else {
        format!("response body redacted ({} bytes)", body.len())
    }
}

impl TxlineError {
    pub(crate) fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    pub(crate) fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    pub(crate) fn proof_decode(message: impl Into<String>) -> Self {
        Self::ProofDecode(message.into())
    }

    pub(crate) fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub(crate) fn solana(message: impl Into<String>) -> Self {
        Self::Solana(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_status_display_redacts_response_body() {
        let secret = "SECRET-API-TOKEN-12345";
        let err = TxlineError::HttpStatus {
            status: 500,
            body: format!("backend echoed token={secret}"),
        };

        let rendered = err.to_string();

        assert!(rendered.contains("HTTP 500"));
        assert!(rendered.contains("redacted"));
        assert!(!rendered.contains(secret));
        assert!(!rendered.contains("backend echoed"));
    }

    #[test]
    fn http_status_debug_redacts_response_body() {
        let secret = "SECRET-API-TOKEN-12345";
        let err = TxlineError::HttpStatus {
            status: 500,
            body: format!("backend echoed token={secret}"),
        };

        let rendered = format!("{err:?}");

        assert!(rendered.contains("HttpStatus"));
        assert!(rendered.contains("redacted"));
        assert!(!rendered.contains(secret));
        assert!(!rendered.contains("backend echoed"));
    }

    #[test]
    fn http_status_keeps_raw_body_for_programmatic_inspection() {
        let err = TxlineError::HttpStatus {
            status: 418,
            body: "raw body".to_owned(),
        };

        match err {
            TxlineError::HttpStatus { status, body } => {
                assert_eq!(status, 418);
                assert_eq!(body, "raw body");
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }
}
