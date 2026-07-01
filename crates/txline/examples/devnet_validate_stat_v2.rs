//! Devnet-only V2 score stat-validation example.
//!
//! Required:
//! - `TXLINE_DEVNET_JWT`
//! - `TXLINE_DEVNET_API_TOKEN`
//! - `TXLINE_FIXTURE_ID`
//! - `TXLINE_SCORE_SEQ`
//! - `TXLINE_STAT_KEYS` comma-separated, for example `1,2,3001,3002`

use std::env;

use txline::validation::{BinaryExpression, Comparison, NDimensionalStrategy, TraderPredicate};
use txline::{ApiToken, GuestJwt, TxlineClient, TxlineConfig};

#[tokio::main]
async fn main() -> txline::Result<()> {
    let Some((jwt, api_token)) = read_auth() else {
        eprintln!("Set TXLINE_DEVNET_JWT and TXLINE_DEVNET_API_TOKEN first.");
        return Ok(());
    };
    let Some((fixture_id, seq, stat_keys)) = read_request() else {
        eprintln!(
            "Set TXLINE_FIXTURE_ID, TXLINE_SCORE_SEQ, and TXLINE_STAT_KEYS from a real score record."
        );
        return Ok(());
    };

    let client = TxlineClient::new(TxlineConfig::devnet())?;
    client.set_guest_jwt(jwt);
    client.set_api_token(api_token);

    let proof = client
        .scores()
        .stat_validation_v2(fixture_id, seq, stat_keys.clone())
        .await?;

    let predicate = TraderPredicate::new(0, Comparison::equal_to());
    let draw_strategy = NDimensionalStrategy::builder(proof.requested_stat_keys().len())
        .binary(0, 1, BinaryExpression::subtract(), predicate)?
        .build()?;

    println!(
        "Requested stat keys, in strategy-index order: {:?}",
        proof.requested_stat_keys()
    );
    println!(
        "V2 proof contains {} stat leaves",
        proof.stats_to_prove().len()
    );
    println!("Example binary draw strategy: {draw_strategy:?}");
    println!("Daily scores epoch day: {}", proof.epoch_day()?);

    Ok(())
}

fn read_auth() -> Option<(GuestJwt, ApiToken)> {
    let jwt = GuestJwt::new(env::var("TXLINE_DEVNET_JWT").ok()?).ok()?;
    let api_token = ApiToken::new(env::var("TXLINE_DEVNET_API_TOKEN").ok()?).ok()?;
    Some((jwt, api_token))
}

fn read_request() -> Option<(i32, i32, Vec<u32>)> {
    let fixture_id = env::var("TXLINE_FIXTURE_ID").ok()?.parse().ok()?;
    let seq = env::var("TXLINE_SCORE_SEQ").ok()?.parse().ok()?;
    let stat_keys = env::var("TXLINE_STAT_KEYS")
        .ok()?
        .split(',')
        .map(str::trim)
        .map(str::parse::<u32>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok()?;
    Some((fixture_id, seq, stat_keys))
}
