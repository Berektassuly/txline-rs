# Validation Golden Fixtures

`validation_golden.devnet.json` stores complete Anchor instruction data bytes for
the Devnet validation builders. Rust tests read this checked-in file with
`include_str!`, so the normal Rust test suite does not require Node, Anchor, or
a local `txodds/tx-on-chain` checkout.

Node, Anchor, and a local upstream checkout are only needed to regenerate or
independently verify the golden file.

## Provenance

The source is Devnet-only and pinned to:

- Repository: `https://github.com/txodds/tx-on-chain`
- Branch / PR: `nojira-re-adding-examples` / PR #3
- Commit: `8dfc6608252f4034a0279b48578c8fe07b949af0`
- IDL path: `examples/devnet/idl/txoracle.json`

The IDL path above is from the PR #3 branch, not `origin/main`. Do not
regenerate these fixtures from the upstream main branch.

Expected bytes are generated independently of the Rust encoder with
`@coral-xyz/anchor`'s `BorshInstructionCoder` and the PR #3 Devnet IDL. Test
payload values such as names, hashes, and IDs are synthetic deterministic test
inputs; the important property is that Anchor generated the expected instruction
bytes from the pinned PR #3 IDL.

## Anchor Version

The generator loads `@coral-xyz/anchor` from the upstream `tx-on-chain`
checkout's `node_modules`, verifies the expected package version, and records
the resolved package version in `validation_golden.devnet.json`.

The latest `@coral-xyz/anchor` npm dist-tag verified for these fixtures is
`0.32.1`, and the generator enforces that version. Install or update
dependencies in the upstream `tx-on-chain` checkout using that checkout's normal
command, such as `npm ci` when its lockfile is present or `npm install`
otherwise. Do not add Node or Anchor dependencies to this Rust crate just to run
the normal test suite.

The generator uses `@coral-xyz/anchor` because that is what the upstream PR #3
branch uses. Do not switch it to another Anchor TypeScript package unless the
upstream branch itself switches. `@anchor-lang/core` is the newer Anchor
TypeScript package name, but it is not used by this PR #3 fixture source.

## Verify

Run from the `txline-rs` repository root:

```powershell
$env:TX_ON_CHAIN_ROOT = "<path-to-local-tx-on-chain-checkout>"
$env:TX_ON_CHAIN_REF = "8dfc6608252f4034a0279b48578c8fe07b949af0"
node crates/txline/tests/fixtures/generate_validation_golden.js --check
```

## Regenerate

```powershell
$env:TX_ON_CHAIN_ROOT = "<path-to-local-tx-on-chain-checkout>"
$env:TX_ON_CHAIN_REF = "8dfc6608252f4034a0279b48578c8fe07b949af0"
node crates/txline/tests/fixtures/generate_validation_golden.js
```
