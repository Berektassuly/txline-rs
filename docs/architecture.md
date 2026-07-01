# Architecture

This SDK is currently a structure-only Rust workspace. The architecture below is
the intended direction, not implemented behavior.

## Layers

| Layer | Modules | Responsibility |
| --- | --- | --- |
| Configuration | `config` | Hold network selection, hosts, Solana RPC, program ID, TxL mint, and guardrails that prevent mixed Mainnet/Devnet values. |
| Credentials | `auth` | Manage guest JWTs, activated API tokens, renewal on 401, and safe header injection. |
| Client facade | `client` | Provide the top-level `TxlineClient` entry point without forcing callers to know every module boundary. |
| Data access | `http` | Map fixtures, odds, and scores REST endpoints from the hosted OpenAPI. |
| Streams | `stream` | Parse SSE, handle heartbeats/no-data periods, reconnect, and resume with `Last-Event-ID`. |
| Solana | `solana` | Derive PDAs, handle Token-2022 accounts, model subscription and purchase flows, and inspect transactions before signing. |
| Validation | `validation` | Decode proofs and build legacy or V2 score-validation payloads for on-chain calls. |

## Expected Flow

1. Choose a network and build a config from one consistent value set.
2. Acquire a guest JWT from the matching host.
3. Prepare Token-2022 accounts and submit `subscribe(serviceLevelId, weeks)`.
4. Sign the activation preimage with the subscribing wallet.
5. Activate the API token on the matching API host.
6. Use both credentials for snapshots, updates, streams, and proof endpoints.
7. For validation, derive PDAs from the proof timestamp and call the matching
   program instruction.

The current crate only exposes placeholder types for the first and third items
in that list.

## Network Values

| Network | API base | Program ID | TxL mint |
| --- | --- | --- | --- |
| Mainnet | `https://txline.txodds.com/api/` | `9ExbZjAapQww1vfcisDmrngPinHTEfpjYRWMunJgcKaA` | `Zhw9TVKp68a1QrftncMSd6ELXKDtpVMNuMGr1jNwdeL` |
| Devnet | `https://txline-dev.txodds.com/api/` | `6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J` | `4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG` |

The SDK should eventually reject configs that mix values from different rows.

## Implementation Boundaries

- `http` should not own wallet secrets or signing.
- `stream` should depend on credential refresh behavior, but should not mint new
  API tokens.
- `solana` should not make off-chain activation decisions.
- `validation` should not fetch arbitrary score data silently; callers should
  pass a real observed `seq`.
- `client` can compose all of the above, but each module should remain usable
  independently for advanced integrations.
