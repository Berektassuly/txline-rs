# Go SDK Notes

## Module Path

The module path is:

```text
github.com/Berektassuly/txline/go
```

This follows the current repository remote/name and keeps the Go SDK as a top-level module without renaming the repository or moving the Rust SDK.

## Public Sources Checked

- `https://txline.txodds.com/docs/docs.yaml`
- `https://txline.txodds.com/documentation/quickstart`
- `https://txline.txodds.com/documentation/programs/addresses`
- `https://txline.txodds.com/documentation/programs/devnet`
- `https://txline.txodds.com/documentation/programs/mainnet`
- `https://txline.txodds.com/documentation/examples/streaming-data`
- `https://txline.txodds.com/documentation/examples/onchain-validation`
- Local Rust SDK and fixtures under `crates/txline`

## Source Conflicts

- OpenAPI `docs.yaml` currently reports `info.version: 1.5.2`.
- Public Devnet and Mainnet IDL pages also report IDL version `1.5.2`.
- The local Rust SDK pins the Devnet IDL source described in `docs/devnet-first.md`: txodds/tx-on-chain PR commit `432b740831c1235ea706784902678381afd241c6`, with golden fixtures generated from that source and `validate_stat_v2` coverage.
- The public Devnet IDL instruction list omits `validate_stat_v2`; the Rust-pinned Devnet source and local golden fixture include it.
- The Go SDK implements `validate_stat_v2` only against the pinned Rust source/golden fixture and keeps this conflict documented rather than treating public IDL pages as resolved.

## Verified Addresses

Devnet:

- Program ID: `6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J`
- TxL mint: `4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG`
- USDT mint: `ELWTKspHKCnCfCiCiqYw1EDH77k8VCP74dK9qytG2Ujh`
- API host: `https://txline-dev.txodds.com`
- API base: `https://txline-dev.txodds.com/api`
- Guest auth: `https://txline-dev.txodds.com/auth/guest/start`

Mainnet public docs list:

- Program ID: `9ExbZjAapQww1vfcisDmrngPinHTEfpjYRWMunJgcKaA`
- TxL mint: `Zhw9TVKp68a1QrftncMSd6ELXKDtpVMNuMGr1jNwdeL`
- USDT mint: `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`

Mainnet transaction helpers are not exposed as parity-supported flows in this SDK.

## Dependency Choice

The only direct non-stdlib dependency is `github.com/gagliardetto/solana-go v1.22.0`.

It is used for Solana public keys, PDA/ATA derivation, generic instruction objects, transaction decoding, message inspection, and signature verification. Those areas are brittle and security-sensitive enough that using a mature Solana Go library is lower risk than hand-rolling wire formats.

Run `gofumpt`, `go mod tidy`, `go test ./...`, `go test -race ./...`,
`go vet ./...`, `staticcheck ./...`, and `govulncheck ./...` after dependency
or behavior changes.

## Safety Model

- No default tests require live TxLINE credentials, wallets, private keys, live RPC, or live API availability.
- Auth wrappers redact JWTs and API tokens in string/debug output.
- SSE streams are cancellable through `context.Context`.
- Raw purchase quote bytes are exposed only through unchecked APIs. Signing/submission paths should use checked purchase quote validation.
- Trading builders intentionally require explicit caller-supplied accounts and do not derive unverified trading PDAs, mints, token programs, or vault accounts.

