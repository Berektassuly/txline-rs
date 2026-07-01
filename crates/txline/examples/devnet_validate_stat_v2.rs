//! Devnet-only V2 score stat-validation example.
//!
//! Required:
//! - `TXLINE_DEVNET_JWT`
//! - `TXLINE_DEVNET_API_TOKEN`
//! - `TXLINE_FIXTURE_ID`
//! - `TXLINE_SCORE_SEQ`
//! - `TXLINE_STAT_KEYS` comma-separated, for example `1,2,3001,3002`
//!
//! Optional:
//! - `TXLINE_VALIDATE_ON_CHAIN=1`
//! - `TXLINE_WALLET` or `ANCHOR_WALLET`

use std::env;

use solana_sdk::signature::read_keypair_file;
use txline::solana::validation::ValidationSimulationConfig;
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

    let stat_count = proof.requested_stat_keys().len();
    if stat_count < 2 {
        eprintln!("Request at least two TXLINE_STAT_KEYS to run the binary/geometric examples.");
        return Ok(());
    }
    let eq = TraderPredicate::new(0, Comparison::equal_to());
    let gt = TraderPredicate::new(0, Comparison::greater_than());
    let lt = TraderPredicate::new(2, Comparison::less_than());

    let single_strategy = NDimensionalStrategy::builder(stat_count)
        .single(0, gt)?
        .build()?;
    let draw_strategy = NDimensionalStrategy::builder(stat_count)
        .binary(0, 1, BinaryExpression::subtract(), eq)?
        .build()?;
    let geometric_strategy = NDimensionalStrategy::builder(stat_count)
        .geometric_target(0, 0)?
        .geometric_target(1, 1)?
        .distance_predicate(lt)
        .build()?;

    println!(
        "Requested stat keys, in strategy-index order: {:?}",
        proof.requested_stat_keys()
    );
    println!(
        "V2 proof contains {} stat leaves",
        proof.stats_to_prove().len()
    );
    println!("Example single strategy: {single_strategy:?}");
    println!("Example binary draw strategy: {draw_strategy:?}");
    println!("Example 2-leg geometric strategy: {geometric_strategy:?}");
    if stat_count >= 3 {
        let three_leg = NDimensionalStrategy::builder(stat_count)
            .binary(0, 1, BinaryExpression::subtract(), eq)?
            .single(2, gt)?
            .build()?;
        println!("Example combined 3-leg strategy: {three_leg:?}");
    }
    if stat_count >= 4 {
        let four_leg = NDimensionalStrategy::builder(stat_count)
            .binary(0, 1, BinaryExpression::subtract(), gt)?
            .single(2, eq)?
            .single(3, lt)?
            .build()?;
        println!("Example combined 4-leg strategy: {four_leg:?}");
    }
    println!("Daily scores epoch day: {}", proof.epoch_day()?);

    if should_simulate() {
        if let Some(wallet_path) = read_wallet_path() {
            let keypair = read_keypair_file(wallet_path).map_err(|err| {
                txline::TxlineError::Solana(format!("could not read wallet keypair: {err}"))
            })?;
            let payload = proof.leading_subset(stat_count.min(2))?;
            let result = client.solana().simulate_validate_stat_v2(
                &keypair,
                &payload,
                &draw_strategy,
                ValidationSimulationConfig::default(),
            )?;
            println!("On-chain validate_stat_v2 simulation returned {result}");
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

fn read_request() -> Option<(i64, i32, Vec<u32>)> {
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

fn should_simulate() -> bool {
    env::var("TXLINE_VALIDATE_ON_CHAIN").is_ok_and(|value| value == "1")
}

fn read_wallet_path() -> Option<String> {
    env::var("TXLINE_WALLET")
        .ok()
        .or_else(|| env::var("ANCHOR_WALLET").ok())
}
