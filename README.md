# txline-rs

Rust SDK scaffold for TxLINE. The intended crate name is `txline`.

> [!WARNING]
> This repository is pre-implementation. It defines workspace shape, feature
> gates, module boundaries, examples, and test placeholders only. It does not
> make HTTP calls, open SSE streams, sign Solana transactions, activate tokens,
> decode proofs, or validate data yet.

## Sources

Keep these as the source of truth while the Rust SDK is being filled in:

| Area | Link |
| --- | --- |
| Hosted docs | <https://txline.txodds.com/documentation/quickstart> |
| OpenAPI | <https://txline.txodds.com/docs/docs.yaml> |
| Program addresses | <https://txline.txodds.com/documentation/programs/addresses> |
| Streaming docs | <https://txline.txodds.com/documentation/examples/streaming-data> |
| On-chain validation docs | <https://txline.txodds.com/documentation/examples/onchain-validation> |
| Devnet examples branch | <https://github.com/txodds/tx-on-chain/tree/nojira-re-adding-examples> |
| Example PR | <https://github.com/txodds/tx-on-chain/pull/3> |
| Docs clarification PR | <https://github.com/txodds/tx-on-chain/pull/4> |

The requested troubleshooting URL,
`https://txline.txodds.com/documentation/examples/troubleshooting`, returned a
Mintlify 404 during this documentation pass. PR #4 adds that page, so the URL
may become valid after the docs PR is published.

## Verified Network Values

Use one row consistently. Do not mix Solana RPC, program ID, TxL mint, guest
JWT host, API host, or activation host across networks.

| Network | Solana RPC | API host | Guest JWT | Program ID | TxL mint |
| --- | --- | --- | --- | --- | --- |
| Mainnet | `https://api.mainnet-beta.solana.com` | `https://txline.txodds.com/api/` | `https://txline.txodds.com/auth/guest/start` | `9ExbZjAapQww1vfcisDmrngPinHTEfpjYRWMunJgcKaA` | `Zhw9TVKp68a1QrftncMSd6ELXKDtpVMNuMGr1jNwdeL` |
| Devnet | `https://api.devnet.solana.com` | `https://txline-dev.txodds.com/api/` | `https://txline-dev.txodds.com/auth/guest/start` | `6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J` | `4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG` |

The OpenAPI `servers` section currently lists production as
`https://txline.txodds.com` and test as `http://txline-dev.txodds.com`, while
the hosted docs and devnet examples use `https://txline-dev.txodds.com`. This
scaffold documents the HTTPS devnet host used by the examples and docs.

## Workspace

| Path | Purpose |
| --- | --- |
| `Cargo.toml` | Workspace manifest. |
| `crates/txline` | Future SDK crate, named `txline`, currently version `0.0.0` and `publish = false`. |
| `examples/` | Non-working example shells for planned devnet flows. |
| `tests/` | Integration test placeholders for activation, PDA derivation, and proof decoding. |
| `docs/` | Scaffold documentation for planned SDK design and integration risks. |

## Feature Flags

The feature flags exist now, but they do not enable implemented behavior yet.

| Feature | Default | Planned scope |
| --- | --- | --- |
| `http` | yes | REST clients, request models, auth headers, retries, and endpoint coverage. |
| `stream` | no | SSE clients for odds and scores; depends on `http`. |
| `solana` | no | Program IDs, Token-2022 accounts, PDAs, subscriptions, purchases, and transaction safety helpers. |
| `validation` | no | Proof decoding, score validation payload builders, and validation strategy helpers; depends on `solana`. |
| `devnet` | yes | Devnet defaults and test-first workflows. |
| `mainnet` | no | Mainnet constants and release readiness checks. |

## Planned Module Map

| Module | Planned responsibility |
| --- | --- |
| `config` | Network-specific hosts, program IDs, mint addresses, and guardrails against mixed-network configuration. |
| `auth` | Guest JWT acquisition/renewal, activated API-token storage, and safe header injection. |
| `client` | User-facing `TxlineClient` entry point that composes auth, HTTP, streams, Solana helpers, and validation. |
| `http` | Fixtures, odds, scores, OpenAPI model mapping, and `/api/scores/stat-validation`. |
| `stream` | SSE parsing, heartbeats, reconnects, `Last-Event-ID`, and no-data periods. |
| `solana` | Token-2022 associated token accounts, subscription transactions, purchase quotes, PDAs, and safety checks before signing. |
| `validation` | Legacy `statKey`/`statKey2`, V2 `statKeys`, proof decoding, timestamp/PDA alignment, and multi-leg strategies. |

## Auth And Activation

TxLINE uses two credentials:

| Credential | Source | Use |
| --- | --- | --- |
| Guest JWT | `POST /auth/guest/start` on the selected network host | Sent as `Authorization: Bearer <jwt>` and included in the activation preimage. |
| Activated API token | `POST /api/token/activate` after a confirmed `subscribe` transaction | Sent as `X-Api-Token` on data requests. |

The OpenAPI description says the guest JWT expires after 30 days. A future SDK
should renew the guest JWT on HTTP 401 from the same network host and retry with
the existing activated API token. It should not reactivate the subscription just
because the guest JWT expired.

Activation signs this exact preimage:

```text
${txSig}:${selectedLeagues.join(",")}:${jwt}
```

For an empty league list, sign:

```text
${txSig}::${jwt}
```

The signing wallet must be the same wallet that submitted the on-chain
`subscribe` transaction. The request body sends `txSig`, the base64 detached
wallet signature, and the selected `leagues` array to `/api/token/activate`.

## Data Access

The hosted OpenAPI currently documents:

| Area | Endpoint families |
| --- | --- |
| Fixtures | `/api/fixtures/snapshot`, `/api/fixtures/updates/...`, `/api/fixtures/validation`, `/api/fixtures/batch-validation` |
| Odds | `/api/odds/snapshot/{fixtureId}`, `/api/odds/updates/...`, `/api/odds/stream`, `/api/odds/validation` |
| Scores | `/api/scores/snapshot/{fixtureId}`, `/api/scores/updates/...`, `/api/scores/historical/{fixtureId}`, `/api/scores/stream`, `/api/scores/stat-validation` |

Most data endpoints require both `Authorization: Bearer <jwt>` and
`X-Api-Token: <api-token>`.

## Streaming Expectations

The future `stream` module should treat TxLINE streams as ordinary SSE:

- Send both credentials and `Accept: text/event-stream`.
- Parse `id`, `event`, `data`, and `retry` fields.
- Keep the connection open through heartbeats and quiet periods.
- Reconnect on transport failures.
- Preserve `Last-Event-ID` when resuming.
- Renew only the guest JWT on 401, then reconnect with the same API token.
- Treat 403 as an entitlement, token, or network mismatch until proven otherwise.

An open stream can be healthy even when there are no covered live fixtures
producing data.

## Validation Notes

`/api/scores/stat-validation` supports two mutually exclusive request modes:

| Mode | Query | Response shape | Planned Rust module |
| --- | --- | --- | --- |
| Legacy | `fixtureId`, `seq`, `statKey`, optional `statKey2` | `ScoresStatValidation` | `validation::legacy` |
| V2 | `fixtureId`, `seq`, comma-separated `statKeys` | `ScoresStatValidationV2` | `validation::v2` |

For V2, requested stat key order matters. The examples map
`statsToProve[index]` to `statProofs[index]`, and strategy indices refer to
those same positions. A strategy using index `2` means the third requested stat
key, not an arbitrary stat key value.

The `seq` parameter must come from a real score record observed through
snapshot, updates, historical data, or the scores stream. Do not use `seq=0` or
a synthetic sequence number.

Validation must align:

- API proof host and program network.
- `summary.updateStats.minTimestamp`, validation timestamp, and PDA epoch day.
- `daily_scores_roots` seed plus epoch day encoded as `u16` little-endian.
- `eventStatRoot`, fixture summary, subtree proof, main-tree proof, and stat
  proofs from the same response.
- Sport-specific phase/status semantics for settlement.

Per the CTO update captured for this scaffold on 2026-07-01, Mainnet and Devnet
are equivalent for the newest V2 score-validation flow. The new Mainnet score
proof behavior applies to score records from `2026-07-01 08:00 GMT` onward.
Older score records may still require legacy proof handling, so the SDK should
remain backward compatible with both old and new score proof formats.

The hosted/generated IDL pages should still be checked before submitting
transactions. This scaffold intentionally records the intended SDK behavior; it
does not ship transaction-building code yet.

## Devnet-First Roadmap

This SDK is devnet-first because it lets maintainers exercise activation,
subscriptions, Token-2022 account setup, SSE reconnect behavior, and validation
payloads without risking mainnet funds.

1. Fill immutable network constants and config validation.
2. Add auth and HTTP transport with explicit secret redaction.
3. Port devnet free-tier activation and data-access flows from PR #3.
4. Add SSE clients and reconnect tests.
5. Add proof decoding and legacy validation payload builders.
6. Add V2 payload and strategy builders.
7. Promote the same API to mainnet constants after parity checks.

Mainnet readiness means mainnet constants are available and the V2 flow is
documented as equivalent, not that this scaffold is safe to use with funds
today.

## Non-Goals For This Phase

- No SDK business logic.
- No Solana signing or transaction submission.
- No purchase quote implementation.
- No HTTP or SSE runtime dependencies.
- No proof parsing or on-chain validation calls.
- No generated OpenAPI models.
- No API stability guarantee.

## Safety Notes

- Never log guest JWTs, activated API tokens, private keys, seed phrases, or
  unredacted request headers.
- Verify purchase quote transactions locally before signing: fee payer, admin
  signature, allowed program IDs, requested amount, and instruction count.
- Do not activate a devnet transaction on the mainnet host or a mainnet
  transaction on the devnet host.
- Free tiers do not require TxL payment, but they still require SOL for Solana
  fees and possible rent.
- Treat 401 as guest-JWT renewal; treat 403 as token, entitlement, expiry, or
  network mismatch.
- Keep old and new score proof formats readable until the historical cutoff is
  no longer relevant to users.

## Future Usage Sketch

This is intentionally non-working pseudocode. It shows the desired shape, not an
implemented API.

```rust,ignore
use txline::{Network, TxlineClient, TxlineConfig};

let config = TxlineConfig::new(Network::Devnet);
let client = TxlineClient::new(config);

// Planned only:
// let jwt = client.auth().guest_start().await?;
// let tx_sig = client.solana().subscribe_free_tier(wallet, 1, 4).await?;
// let api_token = client
//     .auth()
//     .activate(tx_sig, [], wallet.sign_message)
//     .await?;
// let scores = client.http().scores().snapshot(fixture_id).await?;
// let proof = client
//     .validation()
//     .scores_v2(fixture_id, scores[0].seq, [1, 2, 3001, 3002])
//     .await?;
```

See also:

- [Architecture](docs/architecture.md)
- [Devnet First](docs/devnet-first.md)
- [Validation](docs/validation.md)
- [Security](docs/security.md)
