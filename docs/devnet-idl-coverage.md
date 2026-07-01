# Devnet IDL Coverage

This SDK is Devnet-only. Devnet IDL data comes from upstream
`documentation/programs/devnet.mdx`, not the upstream top-level
`idl/txoracle.json`, which currently points at mainnet program
`9ExbZjAapQww1vfcisDmrngPinHTEfpjYRWMunJgcKaA` version `1.4.7`.

Current upstream Devnet docs list program
`6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J` version `1.5.2`. The upstream
PR examples branch `nojira-re-adding-examples` includes a Devnet IDL copy at
the same program ID with version `1.5.5` and `validate_stat_v2`. This SDK
implements the PR parity surface and documents the remaining gaps honestly.

The machine-readable source is `txline::solana::idl::DEVNET_INSTRUCTION_COVERAGE`.

| Instruction | Status | Notes |
| --- | --- | --- |
| `audit_trade_result` | public_flow_planned | Trading audit flow is not yet exposed as a high-level SDK API. |
| `claim_batch_legacy` | public_flow_planned | Claim/refund flows remain planned. |
| `claim_via_resolution` | public_flow_planned | Resolution-claim flow remains planned. |
| `close_intent` | public_flow_planned | Trading intent lifecycle remains planned. |
| `close_pricing_matrix` | admin_only_planned | Admin-only pricing matrix management. |
| `create_intent` | public_flow_planned | Trading intent lifecycle remains planned. |
| `create_trade` | public_flow_planned | Direct trade creation remains planned. |
| `execute_match` | public_flow_planned | Order matching remains planned. |
| `expose_structs` | intentionally_unsupported | IDL/type exposure helper, not an end-user flow. |
| `initialize_pricing_matrix` | admin_only_planned | Admin-only pricing matrix management. |
| `initialize_treasury_v2` | admin_only_planned | Admin-only treasury setup. |
| `initialize_usdt_treasury` | admin_only_planned | Admin-only treasury setup. |
| `insert_batch_root` | admin_only_planned | Oracle root insertion is not exposed to casual SDK users. |
| `insert_fixtures_root` | admin_only_planned | Oracle root insertion is not exposed to casual SDK users. |
| `insert_scores_root` | admin_only_planned | Oracle root insertion is not exposed to casual SDK users. |
| `publish_resolution_root` | admin_only_planned | Oracle resolution root publishing is admin-only. |
| `purchase_subscription_token_usdt` | implemented | Typed builder and quote transaction safety checks are implemented. |
| `refund_batch` | public_flow_planned | Batch refunds remain planned. |
| `request_devnet_faucet` | implemented | Typed builder accepts an explicit faucet tracker account; tracker PDA derivation is not published in the IDL. |
| `settle_matched_trade` | public_flow_planned | Matched trade settlement remains planned. |
| `settle_trade` | public_flow_planned | Direct trade settlement remains planned. |
| `subscribe` | implemented | Subscription transaction builder and high-level setup flow are implemented. |
| `update_pricing_matrix` | admin_only_planned | Admin-only pricing matrix management. |
| `validate_fixture` | implemented | Typed instruction builder and simulation helper are implemented. |
| `validate_fixture_batch` | implemented | Typed instruction builder and simulation helper are implemented. |
| `validate_odds` | implemented | Typed instruction builder and simulation helper are implemented. |
| `validate_stat` | implemented | Typed instruction builder and simulation helper are implemented. |
| `validate_stat_v2` | implemented | Typed instruction builder and simulation helper are implemented for the PR Devnet IDL. |
| `withdraw_usdt` | admin_only_planned | Admin-only treasury withdrawal. |

Live examples require caller-owned wallet/RPC/token inputs and are not run by
default tests. Never log or commit tokens, keypairs, seed phrases, wallet
signatures, or full auth headers.
