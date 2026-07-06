# TxLINE Python SDK Notes

Date: 2026-07-06

## Source Map

Primary public sources checked for this implementation:

- OpenAPI YAML: `https://txline.txodds.com/docs/docs.yaml`
- Quickstart: `https://txline.txodds.com/documentation/quickstart`
- Program addresses: `https://txline.txodds.com/documentation/programs/addresses`
- Devnet IDL page: `https://txline.txodds.com/documentation/programs/devnet`
- Mainnet IDL page: `https://txline.txodds.com/documentation/programs/mainnet`
- Streaming example: `https://txline.txodds.com/documentation/examples/streaming-data`
- On-chain validation example: `https://txline.txodds.com/documentation/examples/onchain-validation`

Local Rust sources remain the SDK behavior baseline for Devnet-only guardrails,
SSE reconnect semantics, validation shape checks, purchase quote safety, and
Anchor/Borsh instruction encoding.

## Source-Of-Truth Conflicts

- OpenAPI reports `openapi: 3.1.0` and `info.version: 1.5.2`.
- Public Devnet and Mainnet IDL pages report `metadata.version: 1.5.2`.
- The Rust SDK pins upstream PR #3 commit
  `432b740831c1235ea706784902678381afd241c6`,
  `examples/devnet/idl/txoracle.json`, with IDL version `1.5.5`.
- OpenAPI documents V2 score stat validation using the `statKeys` query
  parameter and `ScoresStatValidationV2` response shape.
- Public Devnet/Mainnet IDL pages checked on 2026-07-06 did not contain
  `validate_stat_v2` / `validateStatV2`.
- The Python SDK implements `validate_stat_v2` only for byte-for-byte parity
  with the Rust SDK's pinned Devnet PR #3 golden fixture. Mainnet V2 on-chain
  flows are not represented as supported.

Program address facts from public docs matched the Rust constants:

| Value | Devnet | Mainnet |
| --- | --- | --- |
| Program ID | `6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J` | `9ExbZjAapQww1vfcisDmrngPinHTEfpjYRWMunJgcKaA` |
| TxL mint | `4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG` | `Zhw9TVKp68a1QrftncMSd6ELXKDtpVMNuMGr1jNwdeL` |
| USDT mint | `ELWTKspHKCnCfCiCiqYw1EDH77k8VCP74dK9qytG2Ujh` | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` |

The current public Devnet IDL instruction page includes public trading builders
such as `create_intent`, `create_trade`, `execute_match`, `settle_trade`,
`settle_matched_trade`, claims, refunds, `audit_trade_result`, subscription,
purchase, validation, faucet, and admin/root management instructions. The
Python SDK exposes the same conservative low-level public builders as Rust and
does not expose admin/root management helpers as casual high-level flows.

## Dependency Choices

- `httpx` is used for sync and async HTTP, mocked offline tests, and streaming
  primitives.
- `cryptography` is used for Ed25519 signature verification in purchase quote
  safety. This is a heavier dependency than a tiny encoder, but signature
  verification is security-critical and should not be hand-rolled.
- The package does not depend on `solders` or `solana` in this pass. It
  implements small audited Solana primitives for pubkeys, PDAs, account metas,
  instructions, compact transaction parsing, and Anchor/Borsh bytes. Full wallet
  signing/sending UX is intentionally out of scope.
- `pydantic` is not used. DTOs are typed dataclasses with explicit conversion
  helpers to keep runtime dependencies small.

## Intentional Gaps

- No mainnet transaction flows are supported.
- No wallet key management, transaction signing UX, or RPC submission helpers
  are provided.
- Purchase quote checking validates transaction bytes but leaves actual signing
  and broadcast to caller-owned wallet/RPC code.
- Default tests are offline and do not prove live TxLINE API, Solana Devnet RPC,
  or real subscription activation.
