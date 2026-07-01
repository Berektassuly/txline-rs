//! Devnet-only odds validation example.
//!
//! Required:
//! - `TXLINE_DEVNET_JWT`
//! - `TXLINE_DEVNET_API_TOKEN`
//! - `TXLINE_ODDS_MESSAGE_ID`
//! - `TXLINE_ODDS_TS`
//!
//! Optional:
//! - `TXLINE_VALIDATE_ON_CHAIN=1`
//! - `TXLINE_WALLET` or `ANCHOR_WALLET`

use std::env;

use solana_sdk::signature::read_keypair_file;
use txline::solana::validation::ValidationSimulationConfig;
use txline::{ApiToken, GuestJwt, TxlineClient, TxlineConfig};

#[tokio::main]
async fn main() -> txline::Result<()> {
    let Some((jwt, api_token)) = read_auth() else {
        eprintln!("Set TXLINE_DEVNET_JWT and TXLINE_DEVNET_API_TOKEN first.");
        return Ok(());
    };
    let Some((message_id, ts)) = read_request() else {
        eprintln!("Set TXLINE_ODDS_MESSAGE_ID and TXLINE_ODDS_TS from a real odds record.");
        return Ok(());
    };

    let client = TxlineClient::new(TxlineConfig::devnet())?;
    client.set_guest_jwt(jwt);
    client.set_api_token(api_token);

    let proof = client.odds().validation(&message_id, ts).await?;
    println!(
        "Odds proof for fixture {} message {} has {} subtree nodes and {} main-tree nodes",
        proof.summary.fixture_id,
        proof.odds.message_id,
        proof.sub_tree_proof.len(),
        proof.main_tree_proof.len()
    );

    if should_simulate() {
        if let Some(wallet_path) = read_wallet_path() {
            let keypair = read_keypair_file(wallet_path).map_err(|err| {
                txline::TxlineError::Solana(format!("could not read wallet keypair: {err}"))
            })?;
            let result = client.solana().simulate_validate_odds(
                &keypair,
                &proof,
                ValidationSimulationConfig::default(),
            )?;
            println!("On-chain validate_odds simulation returned {result}");
        } else {
            eprintln!("Set TXLINE_WALLET or ANCHOR_WALLET to run on-chain simulation.");
        }
    }

    Ok(())
}

fn read_auth() -> Option<(GuestJwt, ApiToken)> {
    let jwt = GuestJwt::new(env::var("TXLINE_DEVNET_JWT").ok()?).ok()?;
    let api_token = ApiToken::new(env::var("TXLINE_DEVNET_API_TOKEN").ok()?).ok()?;
    Some((jwt, api_token))
}

fn read_request() -> Option<(String, i64)> {
    let message_id = env::var("TXLINE_ODDS_MESSAGE_ID").ok()?;
    let ts = env::var("TXLINE_ODDS_TS").ok()?.parse().ok()?;
    Some((message_id, ts))
}

fn should_simulate() -> bool {
    env::var("TXLINE_VALIDATE_ON_CHAIN").is_ok_and(|value| value == "1")
}

fn read_wallet_path() -> Option<String> {
    env::var("TXLINE_WALLET")
        .ok()
        .or_else(|| env::var("ANCHOR_WALLET").ok())
}
