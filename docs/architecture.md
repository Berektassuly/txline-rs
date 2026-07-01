# Architecture

`txline-rs` is a Devnet-only SDK. The code is organized to keep hosted API
access, Solana transaction construction, and validation payload preparation
separate and reviewable.

## Design Principles

- Keep Devnet constants fixed in one configuration path.
- Prefer typed DTOs and validation helpers over caller-side JSON handling.
- Reject malformed local inputs before sending network requests.
- Keep secret material caller-owned and redacted in debug output.
- Keep transaction helpers conservative until each paid flow is audited end to
  end.

## Layers

| Layer | Modules | Responsibility |
| --- | --- | --- |
| Configuration | `config` | Canonical Devnet hosts, mints, program ID, and RPC override validation. |
| Credentials | `auth`, `client` | Guest JWTs, API tokens, activation preimages, and redacted headers. |
| Data access | `http` | Fixtures, odds, scores, purchase quotes, and validation endpoints. |
| Streams | `stream` | SSE parsing, heartbeat filtering, reconnects, `Last-Event-ID`, and stream-specific JWT refresh on `401`/`403`. |
| Solana | `solana` | Devnet PDAs, Token-2022 ATA derivation/creation, setup, subscription, purchase, faucet, validation, and coverage helpers. |
| Validation | `validation` | Proof decoding, Anchor-compatible stat-validation DTOs, payload conversion, and strategies. |

## Runtime Flows

### Guest and API Credentials

1. Build `TxlineConfig::devnet()`.
2. Construct `TxlineClient::new(cfg)`.
3. Call `start_guest_session()` or set a caller-provided `GuestJwt`.
4. Submit a Devnet `subscribe(service_level_id, weeks)` transaction.
5. Sign the SDK-built activation preimage with the subscribing wallet.
6. Call `activate_subscription(...)` and store the returned `ApiToken`.

`client.devnet_user_setup()` performs the full flow above and also fetches the
pricing matrix, derives and creates the user's Token-2022 ATA when missing, and
waits for RPC visibility before subscribing. If an existing API token is
provided, it skips subscribe and activation.

### REST Access

REST clients are exposed from `TxlineClient`:

- `fixtures()`
- `odds()`
- `scores()`

Authenticated requests automatically retry once with a fresh guest JWT on HTTP
401. REST `403` is left as an entitlement or authorization error. HTTP status
errors preserve the status code and response body.

### Streams

Odds and scores streams use Server-Sent Events. The typed stream wrapper:

- preserves `Last-Event-ID`,
- applies server-provided `retry` backoff hints,
- filters `event: heartbeat` before JSON deserialization,
- refreshes the guest JWT on connection `401` and `403`,
- yields JSON errors for malformed data events.

### Validation

Validation helpers prepare payloads that match the hosted proof responses and
can build Anchor-compatible instructions for:

- `validate_fixture`
- `validate_fixture_batch`
- `validate_odds`
- `validate_stat`
- `validate_stat_v2`

The Solana facade includes simulation helpers that add a compute budget
instruction, simulate the transaction on the configured Devnet RPC, and decode
the program return data as a boolean. V2 payloads preserve requested stat key
order and verify returned stat keys by position before exposing validation
input.

### Paid Purchase Safety

Purchase quote safety checks decode the returned transaction, verify the fee
payer and expected backend signer when configured, limit invoked programs to the
known purchase allowlist, require exactly one
`purchase_subscription_token_usdt` instruction, discriminator-match the
instruction, verify the requested TXLINE amount, and check the expected Devnet
account layout.

## Public Surface

The crate exports a small top-level API:

- `TxlineClient`
- `TxlineConfig`
- `Network`
- `GuestJwt`
- `ApiToken`
- `AuthHeaders`
- `GuestSession`
- `activation_preimage`
- `Result`
- `TxlineError`

Internal modules stay public for the current SDK review phase, but new public
APIs should remain narrow and covered by tests.

Devnet IDL coverage is tracked in `txline::solana::idl` and summarized in
[`docs/devnet-idl-coverage.md`](devnet-idl-coverage.md).

## Out of Scope

- Mainnet constants or feature flags.
- Mainnet RPC support.
- Secret storage or wallet key management.
- Admin/root insertion/update flows in casual examples.
- Trading, settlement, claim, and refund flows that are listed as planned in the
  coverage matrix.
- Live Devnet tests as part of the default test suite.
