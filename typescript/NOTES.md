# TxLINE TypeScript SDK Source Notes

Research date: 2026-07-06.

Primary public sources checked:

- OpenAPI: https://txline.txodds.com/docs/docs.yaml
- Quickstart: https://txline.txodds.com/documentation/quickstart
- Program addresses: https://txline.txodds.com/documentation/programs/addresses
- Public Devnet program page: https://txline.txodds.com/documentation/programs/devnet
- Public Mainnet program page: https://txline.txodds.com/documentation/programs/mainnet
- Streaming example: https://txline.txodds.com/documentation/examples/streaming-data
- On-chain validation example: https://txline.txodds.com/documentation/examples/onchain-validation
- Rust-pinned Devnet IDL source described by `docs/devnet-first.md`: `txodds/tx-on-chain@432b740831c1235ea706784902678381afd241c6`, `examples/devnet/idl/txoracle.json`

## Verified Public Values

OpenAPI reports `openapi: 3.1.0` and `info.version: 1.5.2`.

Devnet:

- Program ID: `6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J`
- TxL mint: `4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG`
- USDT mint: `ELWTKspHKCnCfCiCiqYw1EDH77k8VCP74dK9qytG2Ujh`
- API host: `https://txline-dev.txodds.com`
- Guest auth: `https://txline-dev.txodds.com/auth/guest/start`
- Default RPC: `https://api.devnet.solana.com`

Mainnet reference only:

- Program ID: `9ExbZjAapQww1vfcisDmrngPinHTEfpjYRWMunJgcKaA`
- TxL mint: `Zhw9TVKp68a1QrftncMSd6ELXKDtpVMNuMGr1jNwdeL`
- USDT mint: `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`
- API host: `https://txline.txodds.com`
- Guest auth: `https://txline.txodds.com/auth/guest/start`

This SDK keeps Mainnet as reference data only. Runtime config validation rejects network mixing.

## Conflicts And Decisions

The public Devnet and Mainnet program documentation pages currently describe program metadata around version `1.5.2` and list `validate_stat`, but do not list `validate_stat_v2` / `validateStatV2`.

The Rust SDK is pinned to Devnet IDL source `txoracle 1.5.5` at commit `432b740831c1235ea706784902678381afd241c6`. That pinned IDL includes `validate_stat_v2` and `initialize_treasury_v2`, and Rust golden tests cover the `validate_stat_v2` discriminator `[208, 215, 194, 214, 241, 71, 246, 178]`.

OpenAPI `1.5.2` exposes V2 score stat validation through `/api/scores/stat-validation` using `statKeys` and a `ScoresStatValidationV2` response shape. The TS SDK implements this REST shape and validates stat-key order, `statsToProve` length, and `statProofs` length.

Decision:

- REST V2 `statKeys` support is implemented because it is present in current OpenAPI.
- `validateStatV2Instruction` is implemented with Rust-pinned Devnet IDL parity and golden tests.
- The public-doc conflict is recorded here, and Mainnet `validate_stat_v2` flows are not advertised as supported.

## Current Public Devnet IDL Page Instruction List

The public Devnet program page was inspected and lists:

`audit_trade_result`, `claim_batch_legacy`, `claim_via_resolution`, `close_intent`, `close_pricing_matrix`, `create_intent`, `create_trade`, `execute_match`, `expose_structs`, `initialize_pricing_matrix`, `initialize_usdt_treasury`, `insert_batch_root`, `insert_fixtures_root`, `insert_scores_root`, `publish_resolution_root`, `purchase_subscription_token_usdt`, `refund_batch`, `request_devnet_faucet`, `settle_matched_trade`, `settle_trade`, `subscribe`, `update_pricing_matrix`, `validate_fixture`, `validate_fixture_batch`, `validate_odds`, `validate_stat`, `withdraw_usdt`.

The pinned Rust Devnet IDL adds `initialize_treasury_v2` and `validate_stat_v2`.
