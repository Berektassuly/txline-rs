//! Guest JWTs, activated API tokens, and activation preimages.

use std::fmt;

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

use crate::{Result, TxlineError};

pub const API_TOKEN_HEADER: &str = "X-Api-Token";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuestJwt(String);

impl GuestJwt {
    /// Create a guest JWT, trimming leading and trailing whitespace.
    ///
    /// Empty and whitespace-only values are rejected.
    pub fn new(token: impl Into<String>) -> Result<Self> {
        let token = token.into().trim().to_owned();
        if token.is_empty() {
            return Err(TxlineError::invalid_input("guest JWT must not be empty"));
        }
        Ok(Self(token))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for GuestJwt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GuestJwt(<redacted>)")
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiToken(String);

impl ApiToken {
    /// Create an activated API token, trimming leading and trailing whitespace.
    ///
    /// Empty and whitespace-only values are rejected.
    pub fn new(token: impl Into<String>) -> Result<Self> {
        let token = token.into().trim().to_owned();
        if token.is_empty() {
            return Err(TxlineError::invalid_input("API token must not be empty"));
        }
        Ok(Self(token))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiToken(<redacted>)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestSession {
    pub token: GuestJwt,
}

#[derive(Clone, PartialEq, Eq)]
pub struct AuthHeaders {
    authorization: GuestJwt,
    api_token: Option<ApiToken>,
}

impl AuthHeaders {
    pub fn new(authorization: GuestJwt, api_token: Option<ApiToken>) -> Self {
        Self {
            authorization,
            api_token,
        }
    }

    pub fn to_header_map(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        let auth_value = format!("Bearer {}", self.authorization.as_str());
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_value)?);
        if let Some(api_token) = &self.api_token {
            headers.insert(API_TOKEN_HEADER, HeaderValue::from_str(api_token.as_str())?);
        }
        Ok(headers)
    }

    pub fn has_api_token(&self) -> bool {
        self.api_token.is_some()
    }
}

impl fmt::Debug for AuthHeaders {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthHeaders")
            .field("authorization", &"<redacted>")
            .field("api_token", &self.api_token.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenResponse {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ActivationPayload<'a> {
    #[serde(rename = "txSig")]
    pub tx_sig: &'a str,
    #[serde(rename = "walletSignature")]
    pub wallet_signature: &'a str,
    pub leagues: &'a [i32],
}

/// Build the exact message that must be signed for `/api/token/activate`.
///
/// Empty league lists intentionally produce `txSig::jwt`.
pub fn activation_preimage(
    tx_sig: impl AsRef<str>,
    selected_leagues: &[i32],
    jwt: &GuestJwt,
) -> String {
    let leagues = selected_leagues
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!("{}:{}:{}", tx_sig.as_ref(), leagues, jwt.as_str())
}
