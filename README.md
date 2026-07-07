# txline

Devnet-only multi-language SDK workspace for TxLINE.

[Architecture](docs/architecture.md) |
[Devnet model](docs/devnet-first.md) |
[Validation](docs/validation.md) |
[Devnet IDL coverage](docs/devnet-idl-coverage.md) |
[Security](SECURITY.md) |
[Releasing](docs/releasing.md) |
[Contributing](CONTRIBUTING.md)

> [!IMPORTANT]
> This repository intentionally supports TxLINE Devnet only. Mainnet constants,
> feature flags, examples, and transaction flows are out of scope for every SDK
> package until the Devnet SDK path has been reviewed end to end.

## Overview

`txline` is a monorepo for TxLINE Devnet SDKs. The packages share one Devnet
safety model for configuration, credentials, REST and SSE access, validation
payloads, Solana instruction builders, and paid purchase quote checks.

The Rust crate is the reference implementation for instruction encoding and
golden fixtures. The Go, Python, and TypeScript packages mirror the same
Devnet-only boundaries in language-native APIs.

## SDK Packages

Start with the README for the language you are using:

| Package | Published SDK | Local docs | Path | Checks |
| --- | --- | --- | --- | --- |
| Rust crate | [`txline`](https://crates.io/crates/txline) | [`crates/txline/README.md`](crates/txline/README.md) | [`crates/txline`](crates/txline) | `cargo fmt`, `cargo clippy`, `cargo test` |
| Go module | [`github.com/Berektassuly/txline/go/txline`](https://pkg.go.dev/github.com/Berektassuly/txline/go/txline) | [`go/README.md`](go/README.md) | [`go`](go) | `go test ./...`, `go vet ./...` |
| Python package | [`txline`](https://pypi.org/project/txline/) | [`python/README.md`](python/README.md) | [`python`](python) | `pytest`, `ruff`, `mypy`, `build` |
| TypeScript package | [`@beriktassuly/txline`](https://www.npmjs.com/package/@beriktassuly/txline) | [`typescript/README.md`](typescript/README.md) | [`typescript`](typescript) | `npm run typecheck`, `npm test`, `npm run build` |

## Shared Capabilities

The SDK packages currently include:

- Devnet configuration and client construction.
- Guest JWT acquisition and activated API token handling.
- REST clients for fixtures, odds, scores, validation, and purchase quotes.
- SSE odds and scores streams with reconnect support, `Last-Event-ID`, and
  heartbeat filtering.
- Typed soccer `PlayerStats` support on score records.
- Legacy and V2 score stat-validation DTOs and conversion helpers.
- Proof hash decoding from base64, hex, and byte arrays.
- V2 strategy builders for single, binary, geometric, and distance predicates.
- Devnet PDA helpers and Token-2022 associated token account derivation.
- `subscribe`, `request_devnet_faucet`, `purchase_subscription_token_usdt`,
  and validation instruction builders.
- Low-level public TxODDS trading instruction builders for intents, direct
  trades, matching, settlement, claims, refunds, and audit checks.
- Pricing matrix account decoding and paid quote transaction safety checks.
- A machine-readable Devnet IDL instruction coverage manifest.

## Devnet Configuration

Use the package-specific Devnet factory:

| Language | Devnet factory |
| --- | --- |
| Rust | `TxlineConfig::devnet()` |
| Go | `txline.DevnetConfig()` |
| Python | `TxlineConfig.devnet()` |
| TypeScript | `devnetConfig()` |

Canonical Devnet values:

| Value | Devnet |
| --- | --- |
| API host | `https://txline-dev.txodds.com` |
| API base | `https://txline-dev.txodds.com/api` |
| Guest auth URL | `https://txline-dev.txodds.com/auth/guest/start` |
| Program ID | `6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J` |
| TxL mint | `4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG` |
| USDT mint | `ELWTKspHKCnCfCiCiqYw1EDH77k8VCP74dK9qytG2Ujh` |
| Default RPC | `https://api.devnet.solana.com` |

Custom RPC URLs are allowed for Devnet providers, but every package keeps the
TxLINE program ID and mints fixed to Devnet. Config validation rejects empty RPC
URLs and obvious mainnet-looking RPC overrides. The guard is syntactic; callers
are still responsible for using a real Devnet RPC endpoint.

## Authentication

Data access uses two credentials:

- A guest JWT from `/auth/guest/start`.
- An activated API token from `/api/token/activate` after a confirmed Devnet
  `subscribe(service_level_id, weeks)` transaction.

The shared activation preimage is:

```text
${txSig}:${selectedLeagues.join(",")}:${jwt}
```

For the standard bundle with no selected leagues:

```text
${txSig}::${jwt}
```

Only sign this compatibility-bound message for the matching TxLINE Devnet host,
network, and subscription transaction.

## Safety Model

For paid purchase signing flows, use the checked purchase quote helper in the
language package you are using. It requires the expected backend signer and
returns transaction bytes only after SDK safety validation succeeds.

The public TxODDS trading builders are low-level by design. Callers supply every
trading account explicitly, including token program and vault accounts. The SDKs
do not derive or validate unpublished trading PDAs, manage prediction-market
lifecycles, sign transactions, or send transactions for these flows.

Default tests are offline and do not require live TxLINE Devnet credentials.
Live examples require caller-provided credentials, wallets, and RPC access. Do
not commit JWTs, API tokens, private keys, seed phrases, wallet signatures, or
full auth headers.

Common live-example environment variables:

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

- [Architecture](docs/architecture.md): workspace layout, runtime flows, and
  design boundaries.
- [Devnet model](docs/devnet-first.md): fixed constants, RPC guardrails, and
  intentional non-goals across SDK packages.
- [Validation](docs/validation.md): legacy and V2 score stat-validation payloads
  and golden fixture parity.
- [Devnet IDL coverage](docs/devnet-idl-coverage.md): implemented, planned,
  admin-only, and intentionally unsupported instructions.
- [Security](docs/security.md): secrets, wallet signatures, purchase quotes, and
  stream behavior.
- [Releasing](docs/releasing.md): trusted publishing setup and release workflow
  instructions.
- [Security policy](SECURITY.md): how to report vulnerabilities.
- [Contributing](CONTRIBUTING.md): local workflow and review expectations.

Language-specific usage and examples live in the package READMEs:
[`crates/txline`](crates/txline), [`go`](go), [`python`](python), and
[`typescript`](typescript).

## Development

Run the checks for every package touched by a change. CI runs the same package
checks on pushes to `main` and pull requests.

Rust:

```bash
cargo fmt --check
cargo check --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test
```

Go:

```bash
cd go
go test ./...
go vet ./...
```

Python:

```bash
cd python
python -m pip install -e ".[dev]"
python -m pytest
python -m ruff check .
python -m ruff format --check .
python -m mypy src
python -m build
```

TypeScript:

```bash
cd typescript
npm ci
npm run typecheck
npm test
npm run build
```

Validation instruction tests use checked-in Anchor golden fixtures. Rust, Go,
Python, and TypeScript package tests do not require Anchor or a local
`txodds/tx-on-chain` checkout. See
[`crates/txline/tests/fixtures/README.md`](crates/txline/tests/fixtures/README.md)
for the developer-only verification and regeneration workflow.

## Sources

- TxLINE docs: <https://txline.txodds.com/documentation/quickstart>
- OpenAPI: <https://txline.txodds.com/docs/docs.yaml>
- Program addresses: <https://txline.txodds.com/documentation/programs/addresses>
- Streaming docs: <https://txline.txodds.com/documentation/examples/streaming-data>
- On-chain validation docs: <https://txline.txodds.com/documentation/examples/onchain-validation>
- Devnet IDL docs: <https://github.com/txodds/tx-on-chain/blob/main/documentation/programs/devnet.mdx>
- Devnet PR #3 source commit: <https://github.com/txodds/tx-on-chain/tree/432b740831c1235ea706784902678381afd241c6/examples/devnet>
