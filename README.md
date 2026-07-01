# txline-rs

Devnet-only Rust SDK for TxLINE.

[Architecture](docs/architecture.md) |
[Devnet model](docs/devnet-first.md) |
[Validation](docs/validation.md) |
[Devnet IDL coverage](docs/devnet-idl-coverage.md) |
[Security](SECURITY.md) |
[Contributing](CONTRIBUTING.md)

> [!IMPORTANT]
> This repository intentionally supports TxLINE Devnet only. Mainnet constants,
> feature flags, examples, and transaction flows are out of scope until the
> Devnet SDK path has been reviewed end to end.

## Overview

`txline-rs` provides a small Rust client for the current TxLINE Devnet APIs and
Solana program addresses. It is built around fixed Devnet constants, explicit
credential handling, and conservative transaction and validation helpers.

The crate currently includes:

- Devnet configuration and client construction.
- Guest JWT acquisition and activated API token storage.
- REST clients for fixtures, odds, scores, validation, and purchase quotes.
- SSE odds and scores streams with reconnect support, `Last-Event-ID`, and
  heartbeat filtering. SSE setup refreshes guest JWTs on `401` and `403`.
- Legacy and V2 score stat-validation DTOs and conversion helpers.
- Proof hash decoding from base64, hex, and byte arrays.
- V2 strategy builders for single, binary, geometric, and distance predicates.
- Devnet PDA helpers and Token-2022 associated token account derivation and
  creation.
- Anchor-compatible `subscribe(service_level_id, weeks)`,
  `request_devnet_faucet`, `purchase_subscription_token_usdt`, and validation
  instruction helpers.
- A high-level Devnet setup flow analogous to the upstream TypeScript
  `setupUser` helper.
- Pricing matrix account decoding and paid quote transaction safety checks.
- A machine-readable Devnet IDL instruction coverage manifest.

## Quick Start

```rust,no_run
use txline::{ApiToken, GuestJwt, TxlineClient, TxlineConfig};

# async fn run() -> txline::Result<()> {
let client = TxlineClient::new(TxlineConfig::devnet())?;

let guest = client.start_guest_session().await?;

// After a confirmed Devnet subscribe transaction, sign this message with the
// subscribing wallet and pass the base64 detached signature to activation.
let message = client.activation_preimage("SUBSCRIBE_TX_SIGNATURE", &[])?;

client.set_guest_jwt(GuestJwt::new(guest.token.as_str())?);
client.set_api_token(ApiToken::new("activated-api-token")?);

let fixtures = client.fixtures().snapshot(None, None).await?;
println!("fixtures: {}", fixtures.len());
# Ok(())
# }
```

For the full Devnet setup flow:

```rust,no_run
use txline::{TxlineClient, TxlineConfig};

# async fn run() -> txline::Result<()> {
let client = TxlineClient::new(TxlineConfig::devnet())?;
let setup = client
    .devnet_user_setup()
    .service_level_id(1)
    .weeks(4)
    .selected_leagues(Vec::<i32>::new());
let result = setup.run_with_keypair_path("path/to/devnet-wallet.json").await?;
println!("wallet: {}", result.user_pubkey);
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

## Devnet Configuration

The canonical configuration is `TxlineConfig::devnet()`.

| Value | Devnet |
| --- | --- |
| API host | `https://txline-dev.txodds.com` |
| API base | `https://txline-dev.txodds.com/api` |
| Guest auth URL | `https://txline-dev.txodds.com/auth/guest/start` |
| Program ID | `6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J` |
| TxL mint | `4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG` |
| USDT mint | `ELWTKspHKCnCfCiCiqYw1EDH77k8VCP74dK9qytG2Ujh` |
| Default RPC | `https://api.devnet.solana.com` |

Custom RPC URLs are allowed for Devnet providers:

```rust,no_run
# use txline::TxlineConfig;
let cfg = TxlineConfig::devnet()
    .with_rpc_url("https://custom-rpc.example.com/solana/devnet");
```

Validation rejects empty RPC URLs and obvious mainnet-looking RPC overrides, but
callers are still responsible for providing a real Devnet RPC endpoint.

## Examples

Examples require caller-provided Devnet credentials or an explicit Devnet wallet
path. They do not contain real tokens, signatures, seed phrases, or private
keys. Live on-chain simulations are gated behind environment variables and are
not run by default tests.

```bash
cargo run -p txline --example devnet_free_tier
cargo run -p txline --example devnet_setup_user
cargo run -p txline --example devnet_scores_stream
cargo run -p txline --example devnet_validate_stat
cargo run -p txline --example devnet_validate_stat_v2
cargo run -p txline --example devnet_validate_fixture
cargo run -p txline --example devnet_validate_odds
```

Common environment variables:

```bash
TXLINE_DEVNET_JWT=...
TXLINE_DEVNET_API_TOKEN=...
TXLINE_FIXTURE_ID=17952170
TXLINE_SCORE_SEQ=941
TXLINE_STAT_KEY=1002
TXLINE_STAT_KEYS=1001,1002,1007,2007
TXLINE_WALLET=/path/to/devnet-wallet.json
TXLINE_VALIDATE_ON_CHAIN=1
```

`TXLINE_SCORE_SEQ` must come from a real score record observed through a
snapshot, update endpoint, historical score query, or the scores stream.

## Repository Guide

- [Architecture](docs/architecture.md): crate layout, runtime flows, and design
  boundaries.
- [Devnet model](docs/devnet-first.md): fixed constants, RPC guardrails, and
  intentional non-goals.
- [Validation](docs/validation.md): legacy and V2 score stat-validation payloads.
- [Devnet IDL coverage](docs/devnet-idl-coverage.md): implemented, planned,
  admin-only, and intentionally unsupported instructions.
- [Security](docs/security.md): secrets, wallet signatures, purchase quotes, and
  stream behavior.
- [Security policy](SECURITY.md): how to report vulnerabilities.
- [Contributing](CONTRIBUTING.md): local workflow and review expectations.

## Development

The workspace uses Rust 2024 and currently declares MSRV `1.96`.

```bash
cargo fmt --check
cargo check --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test
```

Normal tests are offline and do not require live TxLINE Devnet credentials.

CI runs the same checks on pushes to `main` and pull requests, plus an MSRV
`cargo check` using the workspace `rust-version`.

## Sources

- TxLINE docs: <https://txline.txodds.com/documentation/quickstart>
- OpenAPI: <https://txline.txodds.com/docs/docs.yaml>
- Program addresses: <https://txline.txodds.com/documentation/programs/addresses>
- Streaming docs: <https://txline.txodds.com/documentation/examples/streaming-data>
- On-chain validation docs: <https://txline.txodds.com/documentation/examples/onchain-validation>
- Devnet IDL docs: <https://github.com/txodds/tx-on-chain/blob/main/documentation/programs/devnet.mdx>
- Devnet examples branch: <https://github.com/txodds/tx-on-chain/tree/nojira-re-adding-examples>
