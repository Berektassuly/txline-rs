//! Devnet-only free-tier data access example.
//!
//! Required for authenticated data calls:
//! - `TXLINE_DEVNET_JWT`
//! - `TXLINE_DEVNET_API_TOKEN`
//!
//! Optional:
//! - `TXLINE_COMPETITION_ID`
//! - `TXLINE_START_EPOCH_DAY`

use std::env;

use txline::{ApiToken, GuestJwt, TxlineClient, TxlineConfig};

#[tokio::main]
async fn main() -> txline::Result<()> {
    let Some((jwt, api_token)) = read_auth() else {
        eprintln!(
            "Set TXLINE_DEVNET_JWT and TXLINE_DEVNET_API_TOKEN after Devnet subscribe + activation."
        );
        return Ok(());
    };

    let client = TxlineClient::new(TxlineConfig::devnet())?;
    client.set_guest_jwt(jwt);
    client.set_api_token(api_token);

    let start_epoch_day = env::var("TXLINE_START_EPOCH_DAY")
        .ok()
        .and_then(|value| value.parse::<u32>().ok());
    let competition_id = env::var("TXLINE_COMPETITION_ID")
        .ok()
        .and_then(|value| value.parse::<i32>().ok());

    let fixtures = client
        .fixtures()
        .snapshot(start_epoch_day, competition_id)
        .await?;
    println!("Fetched {} Devnet fixtures", fixtures.len());
    if let Some(first) = fixtures.first() {
        println!(
            "First fixture: {} vs {} ({})",
            first.participant1, first.participant2, first.fixture_id
        );
    }
    Ok(())
}

fn read_auth() -> Option<(GuestJwt, ApiToken)> {
    let jwt = GuestJwt::new(env::var("TXLINE_DEVNET_JWT").ok()?).ok()?;
    let api_token = ApiToken::new(env::var("TXLINE_DEVNET_API_TOKEN").ok()?).ok()?;
    Some((jwt, api_token))
}
