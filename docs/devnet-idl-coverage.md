# Devnet IDL Coverage

This SDK is Devnet-only. Devnet IDL data comes from upstream
`documentation/programs/devnet.mdx`, not the upstream top-level
`idl/txoracle.json`, which currently points at mainnet program
`9ExbZjAapQww1vfcisDmrngPinHTEfpjYRWMunJgcKaA` version `1.4.7`.

Current upstream Devnet docs list program
`6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J` version `1.5.2`. The upstream
PR examples branch `nojira-re-adding-examples` includes a Devnet IDL copy at
`examples/devnet/idl/txoracle.json` with the same program ID, version `1.5.5`,
and `validate_stat_v2`. This SDK implements the PR parity surface pinned to
commit `8dfc6608252f4034a0279b48578c8fe07b949af0` and documents the remaining
gaps honestly.

The machine-readable source is `txline::solana::idl::DEVNET_INSTRUCTION_COVERAGE`.

The public TxODDS trading flows listed as implemented below are low-level Rust
instruction builders. They require callers to pass every trading account
explicitly and do not derive unverified PDAs, manage a market lifecycle, sign
transactions, or send transactions. Automatic PDA derivation remains limited to
helpers whose seeds have already been verified in this SDK.

| Instruction | Status | Notes |
| --- | --- | --- |
| `audit_trade_result` | implemented | Low-level public trading audit builder with explicit caller-supplied accounts. |
| `claim_batch_legacy` | implemented | Low-level legacy batch claim builder with explicit caller-supplied accounts. |
| `claim_via_resolution` | implemented | Low-level resolution claim builder with explicit caller-supplied accounts. |
| `close_intent` | implemented | Low-level intent close builder with explicit caller-supplied accounts. |
| `close_pricing_matrix` | admin_only_planned | Admin-only pricing matrix management. |
| `create_intent` | implemented | Low-level intent creation builder with explicit caller-supplied accounts. |
| `create_trade` | implemented | Low-level direct trade creation builder with explicit caller-supplied accounts. |
| `execute_match` | implemented | Low-level order match execution builder with explicit caller-supplied accounts. |
| `expose_structs` | intentionally_unsupported | IDL/type exposure helper, not an end-user flow. |
| `initialize_pricing_matrix` | admin_only_planned | Admin-only pricing matrix management. |
| `initialize_treasury_v2` | admin_only_planned | Admin-only treasury setup. |
| `initialize_usdt_treasury` | admin_only_planned | Admin-only treasury setup. |
| `insert_batch_root` | admin_only_planned | Oracle root insertion is not exposed to casual SDK users. |
| `insert_fixtures_root` | admin_only_planned | Oracle root insertion is not exposed to casual SDK users. |
| `insert_scores_root` | admin_only_planned | Oracle root insertion is not exposed to casual SDK users. |
| `publish_resolution_root` | admin_only_planned | Oracle resolution root publishing is admin-only. |
| `purchase_subscription_token_usdt` | implemented | Typed builder and quote transaction safety checks are implemented. |
| `refund_batch` | implemented | Low-level batch refund builder with explicit caller-supplied accounts. |
| `request_devnet_faucet` | implemented | Typed builder accepts an explicit faucet tracker account; tracker PDA derivation is not published in the IDL. |
| `settle_matched_trade` | implemented | Low-level matched trade settlement builder with explicit caller-supplied accounts and proof inputs. |
| `settle_trade` | implemented | Low-level direct trade settlement builder with explicit caller-supplied accounts and proof inputs. |
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
