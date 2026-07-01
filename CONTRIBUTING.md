# Contributing to txline-rs

Thanks for helping improve `txline-rs`. This project is intentionally small and
review-driven, so the best contributions are focused, well-tested, and explicit
about Devnet assumptions.

## Project Scope

The SDK currently supports TxLINE Devnet only.

Do not add mainnet constants, feature flags, transaction flows, or examples
unless the project scope changes first. Devnet program IDs, mints, activation
preimage behavior, and settlement payload semantics should be treated as
security-sensitive.

## Ways to Contribute

- Fix bugs with a regression test.
- Improve documentation when it changes user behavior or clarifies a guardrail.
- Add small validation helpers around existing API responses.
- Improve examples without adding secrets or requiring live credentials in the
  default test suite.
- Review transaction and proof handling for conservative safety improvements.

## Before You Start

For non-trivial changes, open an issue or draft PR first and describe:

- the problem,
- why it matters for Devnet users,
- the smallest proposed change,
- how it will be tested.

Keep refactors separate from behavior changes. Small PRs are easier to review
and safer for SDK users.

## Development Setup

Install the Rust toolchain declared by the workspace:

```bash
rustup toolchain install stable
rustup component add rustfmt clippy
```

From the repository root:

```bash
cargo fmt --check
cargo check --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test
```

Run the full set before asking for review when code changes are involved.

CI runs these checks on pull requests and pushes to `main`. It also runs an MSRV
`cargo check` against the Rust version declared by the workspace.

## Live Devnet Work

Normal tests must not require real TxLINE credentials, wallets, tokens, or SOL.

Live examples may be run manually with explicit environment variables:

```bash
TXLINE_DEVNET_JWT=...
TXLINE_DEVNET_API_TOKEN=...
TXLINE_FIXTURE_ID=...
TXLINE_SCORE_SEQ=...
```

Do not fake live validation. If credentials are unavailable, say that the live
flow was not run.

## Pull Request Expectations

When opening a PR, include:

- what changed,
- why the change is needed,
- tests run,
- whether strict Clippy passed,
- whether live Devnet validation was run or skipped,
- any remaining risks or follow-up work.

If the change affects validation, streams, RPC configuration, activation,
purchase quotes, or Solana transactions, include focused regression tests.

## Documentation

Update documentation with behavior changes. The main entry points are:

- `README.md` for quick start and project map,
- `docs/architecture.md` for module boundaries,
- `docs/devnet-first.md` for network assumptions,
- `docs/security.md` for security boundaries,
- `docs/validation.md` for proof and settlement payload behavior.

## AI-Assisted Contributions

If AI tools materially assisted a contribution, disclose that in the PR and
briefly describe how they were used. Trivial autocomplete does not need a note.

## Security Reports

Do not open public issues for vulnerabilities, credential leaks, or transaction
safety problems. Follow [SECURITY.md](SECURITY.md).
