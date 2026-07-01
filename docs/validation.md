# Validation

This page records the planned validation behavior for the Rust SDK. No proof
decoding or on-chain calls are implemented yet.

## API Request Modes

`/api/scores/stat-validation` supports two mutually exclusive modes:

| Mode | Query shape | Response shape |
| --- | --- | --- |
| Legacy | `fixtureId`, `seq`, `statKey`, optional `statKey2` | `ScoresStatValidation` |
| V2 | `fixtureId`, `seq`, comma-separated `statKeys` | `ScoresStatValidationV2` |

Legacy mode supports one stat or a two-stat predicate. V2 supports dynamic
N-stat validation strategies.

## V2 Positional Mapping

In V2, the order of `statKeys` is part of the contract between the API response
and the strategy:

```text
statKeys=1,2,3001,3002
```

maps to:

| Position | Stat key | Strategy index |
| --- | --- | --- |
| first | `1` | `0` |
| second | `2` | `1` |
| third | `3001` | `2` |
| fourth | `3002` | `3` |

The examples map `statsToProve[index]` to `statProofs[index]`. Any Rust builder
should preserve this positional relationship and make accidental reordering hard.

## Real Sequence Required

The `seq` value must come from a real score record:

- `/api/scores/snapshot/{fixtureId}`
- `/api/scores/updates/{epochDay}/{hourOfDay}/{interval}`
- `/api/scores/updates/{fixtureId}`
- `/api/scores/historical/{fixtureId}`
- `/api/scores/stream`

Do not use `seq=0`, a fixture ID, an array index, or a synthetic sequence.

## Timestamp And PDA Alignment

For score validation, the proof timestamp and PDA must align:

1. Read `targetTs` from `validation.summary.updateStats.minTimestamp`.
2. Compute `epochDay = floor(targetTs / 86400000)`.
3. Derive `daily_scores_roots` with the `epochDay` encoded as `u16`
   little-endian.
4. Pass the same timestamp into the on-chain validation call.

If the timestamp, epoch day, or proof payload comes from a different response,
`InvalidMainTreeProof` or an equivalent proof error is expected.

## Multi-Leg Strategies

V2 strategies can combine:

- single-stat predicates by index,
- binary expressions such as subtracting one indexed stat from another,
- geometric targets over indexed stats,
- shorter slices of a larger proof payload when the selected strategy uses only
  the leading subset of requested stats.

The SDK should keep the requested stat key order visible to callers and should
avoid silently dropping or reordering stats.

## Mainnet And Devnet Status

Per the CTO update captured for this scaffold on 2026-07-01, Mainnet and Devnet
are equivalent for the newest V2 score-validation flow. The new Mainnet score
proof behavior applies to score records from `2026-07-01 08:00 GMT` onward.

Older score records may use legacy proof formats. The SDK should support both
legacy and new proof shapes until the project explicitly drops historical
compatibility.

## Settlement Semantics

Validation proves that a stat existed in a score record committed through the
Merkle proof path. It does not decide whether a record is the right settlement
record for a market.

Callers must choose a score record whose sport-specific phase/status matches the
condition they want to settle. An in-running first-half record proves the value
at that moment; it does not prove the final first-half result unless that record
corresponds to the completed phase.
