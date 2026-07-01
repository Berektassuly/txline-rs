//! Devnet-only free-tier data access example.
//!
//! Required for authenticated data calls:
//! - either `TXLINE_DEVNET_JWT` and `TXLINE_DEVNET_API_TOKEN`
//! - or `TXLINE_WALLET`/`ANCHOR_WALLET` to perform Devnet setup
//!
//! Optional:
//! - `TXLINE_RPC_URL`
//! - `TXLINE_SERVICE_LEVEL_ID` default `1`
//! - `TXLINE_SUBSCRIPTION_WEEKS` default `4`
//! - `TXLINE_SELECTED_LEAGUES` comma-separated league IDs
//! - `TXLINE_COMPETITION_ID`
//! - `TXLINE_START_EPOCH_DAY`

use std::env;

use txline::{ApiToken, GuestJwt, TxlineClient, TxlineConfig};

#[tokio::main]
async fn main() -> txline::Result<()> {
    let config = match env::var("TXLINE_RPC_URL") {
        Ok(rpc_url) => TxlineConfig::devnet().with_rpc_url(rpc_url),
        Err(_) => TxlineConfig::devnet(),
    };
    let client = TxlineClient::new(config)?;
    if let Some((jwt, api_token)) = read_auth() {
        client.set_guest_jwt(jwt);
        client.set_api_token(api_token);
    } else if let Some(wallet_path) = read_wallet_path() {
        let service_level_id = env::var("TXLINE_SERVICE_LEVEL_ID")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(1);
        let weeks = env::var("TXLINE_SUBSCRIPTION_WEEKS")
            .ok()
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(4);
        let setup = client
            .devnet_user_setup()
            .service_level_id(service_level_id)
            .weeks(weeks)
            .selected_leagues(read_selected_leagues());
        let result = setup.run_with_keypair_path(wallet_path).await?;
        println!(
            "Devnet setup ready for wallet {} using {} pricing rows",
            result.user_pubkey,
            result.pricing_matrix.rows.len()
        );
        if let Some(signature) = result.subscribe_signature {
            println!("Subscribe transaction confirmed: {signature}");
        }
    } else {
        eprintln!(
            "Set TXLINE_DEVNET_JWT and TXLINE_DEVNET_API_TOKEN, or set TXLINE_WALLET/ANCHOR_WALLET to run setup."
        );
        return Ok(());
    }

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

fn read_wallet_path() -> Option<String> {
    env::var("TXLINE_WALLET")
        .ok()
        .or_else(|| env::var("ANCHOR_WALLET").ok())
}

fn read_selected_leagues() -> Vec<i32> {
    env::var("TXLINE_SELECTED_LEAGUES")
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .filter_map(|value| value.parse::<i32>().ok())
                .collect()
        })
        .unwrap_or_default()
}
