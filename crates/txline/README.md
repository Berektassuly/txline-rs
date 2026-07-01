# txline

Devnet-only Rust SDK for TxLINE.

> This crate intentionally supports TxLINE Devnet only. Mainnet constants,
> feature flags, examples, and transaction flows are out of scope for this SDK
> version.

## Overview

`txline` provides typed Rust helpers for the current TxLINE Devnet APIs and
Solana program addresses:

- Devnet configuration and client construction.
- Guest JWT and activated API token handling.
- REST clients for fixtures, odds, scores, validation, and purchase quotes.
- SSE odds and scores streams with reconnect support and heartbeat filtering.
- Legacy and V2 score stat-validation DTOs.
- Devnet PDA and `subscribe(service_level_id, weeks)` transaction helpers.

## Quick Start

```rust,no_run
use txline::{ApiToken, GuestJwt, TxlineClient, TxlineConfig};

# async fn run() -> txline::Result<()> {
let client = TxlineClient::new(TxlineConfig::devnet())?;

let guest = client.start_guest_session().await?;
let message = client.activation_preimage("SUBSCRIBE_TX_SIGNATURE", &[])?;

client.set_guest_jwt(GuestJwt::new(guest.token.as_str())?);
client.set_api_token(ApiToken::new("activated-api-token")?);

let fixtures = client.fixtures().snapshot(None, None).await?;
println!("fixtures: {}", fixtures.len());
# Ok(())
# }
```

The activation preimage is:

```text
${txSig}:${selectedLeagues.join(",")}:${jwt}
```

For the standard bundle with no selected leagues:

```text
${txSig}::${jwt}
```

## Documentation

- Repository: <https://github.com/Berektassuly/txline-rs>
- API docs: <https://docs.rs/txline>
- TxLINE docs: <https://txline.txodds.com/documentation/quickstart>

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.
