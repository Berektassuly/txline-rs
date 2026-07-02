use reqwest::header::AUTHORIZATION;
use txline::auth::API_TOKEN_HEADER;
use txline::{ApiToken, AuthHeaders, GuestJwt, TxlineClient, TxlineConfig, activation_preimage};

#[test]
fn activation_preimage_preserves_empty_league_slot() {
    let jwt = GuestJwt::new("jwt-value").unwrap();
    assert_eq!(activation_preimage("txSig", &[], &jwt), "txSig::jwt-value");
}

#[test]
fn activation_preimage_joins_leagues_in_order() {
    let jwt = GuestJwt::new("jwt-value").unwrap();
    assert_eq!(
        activation_preimage("txSig", &[501, 804, 202], &jwt),
        "txSig:501,804,202:jwt-value"
    );
}

#[test]
fn auth_debug_redacts_secrets() {
    let client = TxlineClient::new(TxlineConfig::devnet()).unwrap();
    client.set_guest_jwt(GuestJwt::new("secret-jwt").unwrap());
    client.set_api_token(ApiToken::new("secret-api-token").unwrap());

    let headers = client.auth_headers(true).unwrap();
    let debug = format!("{headers:?}");

    assert!(!debug.contains("secret-jwt"));
    assert!(!debug.contains("secret-api-token"));
    assert!(debug.contains("<redacted>"));
    assert!(headers.has_api_token());
}

#[test]
fn token_constructors_trim_surrounding_whitespace_before_headers() {
    let jwt = GuestJwt::new(" \tguest.jwt\n").unwrap();
    let api_token = ApiToken::new("\napi-token \t").unwrap();

    assert_eq!(jwt.as_str(), "guest.jwt");
    assert_eq!(api_token.as_str(), "api-token");

    let headers = AuthHeaders::new(jwt, Some(api_token))
        .to_header_map()
        .unwrap();
    assert_eq!(headers[AUTHORIZATION], "Bearer guest.jwt");
    assert_eq!(headers[API_TOKEN_HEADER], "api-token");
}

#[test]
fn token_constructors_reject_whitespace_only_values() {
    assert!(GuestJwt::new(" \t\n").is_err());
    assert!(ApiToken::new(" \t\n").is_err());
}
