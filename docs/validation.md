# Validation

The SDK packages keep network calls, response decoding, instruction
construction, and on-chain simulation separate so settlement-oriented code can
review each step.

Devnet score validation payloads use `ScoresBatchSummary.fixture_id: i64`,
matching the Devnet IDL. HTTP fixture IDs may be small in current examples, but
Anchor-compatible instruction args must not narrow them to `i32`.

## Legacy

Legacy mode uses one or two stat keys:

```text
fixtureId=...&seq=...&statKey=...
fixtureId=...&seq=...&statKey=...&statKey2=...
```

The SDKs return language-native `ScoresStatValidation` values with
`statToProve`, `statProof`, and optional second-stat fields.

## V2

V2 mode accepts an ordered list of stat keys:

```text
fixtureId=...&seq=...&statKeys=1001,1002,1007,2007
```

Requested stat key order is preserved in `ScoresStatValidationV2`.
Settlement-oriented strategies refer to indices in that preserved order.

The SDKs check:

- `seq > 0`,
- every proof hash decodes to exactly 32 bytes,
- `statsToProve.len() == requested_stat_keys.len()`,
- `statsToProve[i].key == requested_stat_keys[i]`,
- `statProofs.len() == statsToProve.len()`.

## Strategy Builder

`NDimensionalStrategy::builder(stat_count)` supports:

- single-stat predicates by index,
- binary predicates using add or subtract,
- geometric targets,
- distance predicates.

The builder rejects out-of-bounds indices before malformed strategy data can be
submitted.

Tests and examples cover single-stat, binary, geometric two-leg, combined
three-leg, and combined four-leg V2 strategy shapes.

## On-Chain Simulation

The Rust Solana facade can build and simulate Anchor-compatible instructions
for:

- `validate_fixture`
- `validate_fixture_batch`
- `validate_odds`
- `validate_stat`
- `validate_stat_v2`

Simulation helpers add a compute budget instruction with a default limit of
`1_400_000` units, simulate against the configured Devnet RPC, and decode the
program return data as a Borsh boolean. They do not fake validation by checking
local DTO shape only.

Live simulation examples are gated:

```bash
TXLINE_VALIDATE_ON_CHAIN=1
TXLINE_WALLET=/path/to/devnet-wallet.json
cargo run -p txline --example devnet_validate_stat
cargo run -p txline --example devnet_validate_stat_v2
cargo run -p txline --example devnet_subscription_scores_1stat
cargo run -p txline --example devnet_subscription_scores_v2
cargo run -p txline --example devnet_subscription_scores_v2a
cargo run -p txline --example devnet_validate_fixture
cargo run -p txline --example devnet_validate_odds
```

The Rust `devnet_subscription_scores_*` examples mirror the upstream PR #3
`subscription_scores_1stat.ts`, `subscription_scores_v2.ts`, and
`subscription_scores_v2a.ts` flows while keeping caller credentials and live
simulation gated by environment variables.

Default tests in all packages cover deterministic instruction encoding and
payload behavior, but do not require live Devnet availability.

Checked-in Anchor golden fixtures live in
`crates/txline/tests/fixtures/validation_golden.devnet.json` and are reused by
the Rust, Go, Python, and TypeScript tests. Normal tests do not require Anchor
or a local `txodds/tx-on-chain` checkout. Those tools are only needed to
regenerate or independently verify the golden file, using the fixture README
workflow and the PR #3 Devnet IDL pinned to commit
`432b740831c1235ea706784902678381afd241c6`.

## Sequence Source

`seq` must come from a real score record from snapshot, updates, historical
scores, or the scores stream. Do not use `seq=0`, fixture IDs, array positions,
or synthetic sequence numbers.

## Testing

Regression tests across the SDK packages cover:

- proof hash decoding,
- positive `seq` validation,
- V2 stat/proof length checks,
- V2 stat key order checks,
- strategy index bounds.
- `i64` fixture ID preservation in validation payloads,
- validation instruction discriminator and fixture-ID encoding,
- V2 strategy shape coverage.
