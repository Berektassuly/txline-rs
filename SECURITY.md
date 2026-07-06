# Security Policy

`txline` is a Devnet-only SDK. Security issues are still important because
the SDK packages handle credentials, wallet signatures, RPC endpoints, Solana
transactions, and validation payloads.

## Supported Scope

The current supported scope is the repository's default branch and the
Devnet-only Rust, Go, Python, and TypeScript SDK packages.

Mainnet behavior is not supported by this SDK version.

## Reporting a Vulnerability

Do not open a public issue for suspected vulnerabilities.

Use the repository's private security reporting channel when available. If that
is not configured, contact the maintainers out of band and share only the
minimum information needed to establish impact.

Please include:

- affected component,
- reproduction steps,
- expected impact,
- whether credentials, wallet signatures, or private keys were exposed,
- whether a live Devnet flow was involved.

Do not include real private keys, seed phrases, full JWTs, full API tokens, or
wallet signatures in the report body.

## Sensitive Areas

Treat changes in these areas as security-sensitive:

- `auth` and credential redaction in every package,
- activation preimage construction,
- RPC override helpers and network guardrails,
- Solana transaction construction and quote handling,
- proof decoding and V2 stat-validation payload conversion,
- SSE reconnect and typed event parsing.

## Responsible Testing

Normal tests should remain offline and credential-free. Live Devnet validation
must be opt-in through explicit environment variables and should be reported as
run or skipped.
