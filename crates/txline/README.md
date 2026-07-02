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
- Devnet PDA and Token-2022 ATA helpers.
- High-level Devnet setup flow for wallet, pricing matrix, subscribe, activation,
  and API token storage.
- `subscribe`, `request_devnet_faucet`, `purchase_subscription_token_usdt`, and
  on-chain validation instruction builders.
- Low-level public TxODDS trading builders for intents, direct trades, matching,
  settlement, claims, refunds, and audit checks.
- View-like validation simulation helpers for fixtures, odds, legacy stats, and
  V2 stats.
- Paid purchase quote transaction safety checks.

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

Wallet setup is available through `client.devnet_user_setup()`. It fetches the
pricing matrix, ensures the user's Token-2022 ATA exists, submits
`subscribe(service_level_id, weeks)`, signs the activation preimage, calls
`/api/token/activate`, and stores the returned API token on the client. If an
existing API token is supplied, on-chain subscribe and activation are skipped.

The activation preimage is:

```text
${txSig}:${selectedLeagues.join(",")}:${jwt}
```

For the standard bundle with no selected leagues:

```text
${txSig}::${jwt}
```

## Trading Builders

`txline::solana::trading` includes typed builders for the public, non-admin
TxODDS Devnet trading instructions in the pinned PR #3 IDL (`1.5.5`). They are
explicit-account instruction builders only; callers remain responsible for PDA
selection, market lifecycle orchestration, signing, simulation, and sending.

## Documentation

- Repository: <https://github.com/Berektassuly/txline-rs>
- API docs: <https://docs.rs/txline>
- TxLINE docs: <https://txline.txodds.com/documentation/quickstart>
- Devnet IDL docs: <https://github.com/txodds/tx-on-chain/blob/main/documentation/programs/devnet.mdx>
- Devnet PR examples source: <https://github.com/txodds/tx-on-chain/tree/nojira-re-adding-examples>

Normal Rust tests use checked-in validation golden fixtures and do not require
Node, Anchor, or a local `txodds/tx-on-chain` checkout.

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.
