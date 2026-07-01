//! Devnet setupUser analogue.
//!
//! Required:
//! - `TXLINE_WALLET` or `ANCHOR_WALLET`
//!
//! Optional:
//! - `TXLINE_RPC_URL`
//! - `TXLINE_SERVICE_LEVEL_ID` default `1`
//! - `TXLINE_SUBSCRIPTION_WEEKS` default `4`
//! - `TXLINE_SELECTED_LEAGUES` comma-separated league IDs
//! - `TXLINE_DEVNET_JWT`
//! - `TXLINE_DEVNET_API_TOKEN` to bypass subscribe + activation

use std::env;

use txline::{ApiToken, GuestJwt, TxlineClient, TxlineConfig};

#[tokio::main]
async fn main() -> txline::Result<()> {
    let Some(wallet_path) = read_wallet_path() else {
        eprintln!("Set TXLINE_WALLET or ANCHOR_WALLET to a Devnet wallet keypair JSON file.");
        return Ok(());
    };

    let config = match env::var("TXLINE_RPC_URL") {
        Ok(rpc_url) => TxlineConfig::devnet().with_rpc_url(rpc_url),
        Err(_) => TxlineConfig::devnet(),
    };
    let client = TxlineClient::new(config)?;
    let mut setup = client
        .devnet_user_setup()
        .service_level_id(read_service_level_id())
        .weeks(read_weeks())
        .selected_leagues(read_selected_leagues());

    if let Some(jwt) = env::var("TXLINE_DEVNET_JWT")
        .ok()
        .and_then(|value| GuestJwt::new(value).ok())
    {
        setup = setup.existing_guest_jwt(jwt);
    }
    if let Some(api_token) = env::var("TXLINE_DEVNET_API_TOKEN")
        .ok()
        .and_then(|value| ApiToken::new(value).ok())
    {
        setup = setup.existing_api_token(api_token);
    }

    let result = setup.run_with_keypair_path(wallet_path).await?;
    println!("Wallet: {}", result.user_pubkey);
    println!("User TXL Token-2022 ATA: {}", result.user_txl_ata);
    println!("Pricing rows fetched: {}", result.pricing_matrix.rows.len());
    if let Some(signature) = result.subscribe_signature {
        println!("Subscribe transaction confirmed: {signature}");
    } else {
        println!("Existing API token supplied; subscribe and activation were skipped.");
    }
    println!("Client now has an activated API token stored in memory.");
    Ok(())
}

fn read_wallet_path() -> Option<String> {
    env::var("TXLINE_WALLET")
        .ok()
        .or_else(|| env::var("ANCHOR_WALLET").ok())
}

fn read_service_level_id() -> u16 {
    env::var("TXLINE_SERVICE_LEVEL_ID")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1)
}

fn read_weeks() -> u8 {
    env::var("TXLINE_SUBSCRIPTION_WEEKS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(4)
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
