//! Devnet-only legacy score stat-validation example.
//!
//! Required:
//! - `TXLINE_DEVNET_JWT`
//! - `TXLINE_DEVNET_API_TOKEN`
//! - `TXLINE_FIXTURE_ID`
//! - `TXLINE_SCORE_SEQ`
//! - `TXLINE_STAT_KEY`
//!
//! Optional:
//! - `TXLINE_STAT_KEY2`

use std::env;

use txline::{ApiToken, GuestJwt, TxlineClient, TxlineConfig};

#[tokio::main]
async fn main() -> txline::Result<()> {
    let Some((jwt, api_token)) = read_auth() else {
        eprintln!("Set TXLINE_DEVNET_JWT and TXLINE_DEVNET_API_TOKEN first.");
        return Ok(());
    };
    let Some((fixture_id, seq, stat_key, stat_key2)) = read_request() else {
        eprintln!(
            "Set TXLINE_FIXTURE_ID, TXLINE_SCORE_SEQ, and TXLINE_STAT_KEY from a real score record."
        );
        return Ok(());
    };

    let client = TxlineClient::new(TxlineConfig::devnet())?;
    client.set_guest_jwt(jwt);
    client.set_api_token(api_token);

    let proof = client
        .scores()
        .stat_validation_legacy(fixture_id, seq, stat_key, stat_key2)
        .await?;
    println!(
        "Legacy proof for fixture {} seq {} has {} stat proof nodes; epoch day {}",
        fixture_id,
        seq,
        proof.stat_proof.len(),
        proof.epoch_day()?
    );
    Ok(())
}

fn read_auth() -> Option<(GuestJwt, ApiToken)> {
    let jwt = GuestJwt::new(env::var("TXLINE_DEVNET_JWT").ok()?).ok()?;
    let api_token = ApiToken::new(env::var("TXLINE_DEVNET_API_TOKEN").ok()?).ok()?;
    Some((jwt, api_token))
}

fn read_request() -> Option<(i32, i32, u32, Option<u32>)> {
    let fixture_id = env::var("TXLINE_FIXTURE_ID").ok()?.parse().ok()?;
    let seq = env::var("TXLINE_SCORE_SEQ").ok()?.parse().ok()?;
    let stat_key = env::var("TXLINE_STAT_KEY").ok()?.parse().ok()?;
    let stat_key2 = env::var("TXLINE_STAT_KEY2")
        .ok()
        .and_then(|value| value.parse().ok());
    Some((fixture_id, seq, stat_key, stat_key2))
}
