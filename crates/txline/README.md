# txline

Devnet-only Rust SDK for TxLINE.

This crate is the Rust package in the `txline` multi-language SDK workspace.

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
- Typed soccer `PlayerStats` support on score records.
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

Published crate: <https://crates.io/crates/txline>

For paid purchase signing flows, use `TxlineClient::purchase_quote_checked`.
It requires the expected backend signer and returns transaction bytes only after
SDK safety validation succeeds.

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

Only sign this compatibility-bound message for the matching TxLINE Devnet host,
network, and subscription transaction.

## Trading Builders

`txline::solana::trading` includes typed builders for the public, non-admin
TxODDS Devnet trading instructions in the pinned PR #3 IDL (`1.5.5`). They are
explicit-account instruction builders only; callers remain responsible for PDA
selection and review. The builders do not validate mints, token programs, vault
accounts, or PDA derivations, and callers remain responsible for market
lifecycle orchestration, signing, simulation where appropriate, and sending.

The Devnet V2 score examples include Rust counterparts for the upstream PR #3
`subscription_scores_1stat.ts`, `subscription_scores_v2.ts`, and
`subscription_scores_v2a.ts` flows.

## World Cup Trading Lifecycle

`txline::trading_lifecycle` adds a Rust helper layer for World Cup-style
prediction-market demos that settle from TxLINE score data. It composes the
published Devnet pieces rather than introducing a private trading API:

1. Subscribe or use the World Cup free tier, start a guest session, and activate
   an API token. Data requests require both `Authorization: Bearer <guest-jwt>`
   and `X-Api-Token: <activated-api-token>`.
2. Define `ScoreMarketTerms` for final outcome, total-goals, or spread-style
   markets. Final-outcome soccer defaults use `period=100`, participant 1 goals
   stat key `1`, and participant 2 goals stat key `2`.
3. Pass an explicit `TermsHash` into `create_intent_plan` or
   `create_trade_plan`. The public Devnet docs and IDL do not define a
   production terms-hash preimage, so the coordinating application or backend
   owns that hash format.
4. Build intent, direct trade, match, close, settlement, claim, refund, and audit
   plans from caller-supplied accounts. The SDK does not derive unpublished
   trading PDAs or choose vaults.
5. Observe live or historical scores through the REST clients or scores stream.
   Final outcome detection requires `action=game_finalised`, `statusId=100`, and
   `period=100`.
6. Fetch the V2 proof payload with `stat_validation_v2(fixture_id, seq,
   stat_keys)`. The helper checks that proof stat-key order matches the market
   order before building a validation instruction.
7. Build and simulate or submit `validate_stat_v2`, then use the caller-owned
   trade accounts to settle, claim, refund, or audit where the public Devnet IDL
   supports it.

```rust,no_run
use txline::{
    ApiToken, FinalOutcomeConfig, GuestJwt, TxlineClient, TxlineConfig,
    extract_final_outcome, final_outcome_validation_plan, is_final_outcome_record,
};
use txline::solana::pda::parse_pubkey;

# async fn run() -> txline::Result<()> {
let client = TxlineClient::new(TxlineConfig::devnet())?;
client.set_guest_jwt(GuestJwt::new("guest-jwt")?);
client.set_api_token(ApiToken::new("activated-api-token")?);

let fixture_id = 17_952_170;
let scores = client.scores().historical_by_fixture(fixture_id).await?;
let Some(final_score) = scores.iter().find(|score| is_final_outcome_record(score)) else {
    return Ok(());
};
let outcome = extract_final_outcome(final_score, FinalOutcomeConfig::soccer_default())?;

let validation = client
    .scores()
    .stat_validation_v2(outcome.fixture_id, outcome.seq, outcome.stat_keys())
    .await?;

let program_id = parse_pubkey(client.config().program_id.as_str())?;
let plan = final_outcome_validation_plan(program_id, &validation, &outcome)?;
let validation_ix = plan.instructions[0].clone();
# let _ = validation_ix;
# Ok(())
# }
```

For direct `settle_trade` and `settle_matched_trade` helpers, the current Devnet
IDL uses the legacy score-proof shape. Use
`settle_trade_params_from_legacy_validation` or
`settle_matched_trade_params_from_legacy_validation` after fetching the matching
legacy `stat_validation_legacy` proof, or pass fully reviewed low-level params
yourself.

Useful references:

- World Cup hackathon: <https://superteam.fun/earn/hackathon/world-cup/>
- On-chain validation: <https://txline.txodds.com/documentation/examples/onchain-validation>
- Streaming data: <https://txline.txodds.com/documentation/examples/streaming-data>
- Devnet IDL JSON: <https://github.com/txodds/tx-on-chain/blob/main/examples/devnet/idl/txoracle.json>

## Documentation

- Repository: <https://github.com/Berektassuly/txline>
- Published crate: <https://crates.io/crates/txline>
- API docs: <https://docs.rs/txline>
- TxLINE docs: <https://txline.txodds.com/documentation/quickstart>
- Devnet IDL docs: <https://github.com/txodds/tx-on-chain/blob/main/documentation/programs/devnet.mdx>
- Devnet PR #3 source commit: <https://github.com/txodds/tx-on-chain/tree/432b740831c1235ea706784902678381afd241c6/examples/devnet>

Normal Rust tests use checked-in validation golden fixtures and do not require
Node, Anchor, or a local `txodds/tx-on-chain` checkout.

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.
