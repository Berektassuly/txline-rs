//! Devnet-only scores SSE example.
//!
//! Required:
//! - `TXLINE_DEVNET_JWT`
//! - `TXLINE_DEVNET_API_TOKEN`
//!
//! Optional:
//! - `TXLINE_FIXTURE_ID`
//! - `TXLINE_STREAM_SECONDS` (default: 30)

use std::{env, time::Duration};

use futures_util::StreamExt;
use txline::stream::StreamOptions;
use txline::{ApiToken, GuestJwt, TxlineClient, TxlineConfig};

#[tokio::main]
async fn main() -> txline::Result<()> {
    let Some((jwt, api_token)) = read_auth() else {
        eprintln!("Set TXLINE_DEVNET_JWT and TXLINE_DEVNET_API_TOKEN to open the Devnet stream.");
        return Ok(());
    };

    let client = TxlineClient::new(TxlineConfig::devnet())?;
    client.set_guest_jwt(jwt);
    client.set_api_token(api_token);

    let fixture_id = env::var("TXLINE_FIXTURE_ID")
        .ok()
        .and_then(|value| value.parse::<i64>().ok());
    let seconds = env::var("TXLINE_STREAM_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30);

    let mut stream = client.scores_stream().stream(StreamOptions {
        fixture_id,
        ..StreamOptions::default()
    });

    let deadline = tokio::time::sleep(Duration::from_secs(seconds));
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            _ = &mut deadline => break,
            item = stream.next() => {
                match item {
                    Some(Ok(event)) => println!("score event {:?}: seq {}", event.id, event.data.seq),
                    Some(Err(err)) => eprintln!("stream error: {err}"),
                    None => break,
                }
            }
        }
    }

    Ok(())
}

fn read_auth() -> Option<(GuestJwt, ApiToken)> {
    let jwt = GuestJwt::new(env::var("TXLINE_DEVNET_JWT").ok()?).ok()?;
    let api_token = ApiToken::new(env::var("TXLINE_DEVNET_API_TOKEN").ok()?).ok()?;
    Some((jwt, api_token))
}
