# Devnet First

The crate supports TxLINE Devnet only in this implementation phase.

## IDL Source

Use upstream `documentation/programs/devnet.mdx` as the Devnet IDL source, not
the top-level upstream `idl/txoracle.json`. The top-level file currently points
at mainnet address `9ExbZjAapQww1vfcisDmrngPinHTEfpjYRWMunJgcKaA` and version
`1.4.7`.

The Devnet docs currently identify program
`6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J` and IDL version `1.5.2`. The
upstream PR examples branch `nojira-re-adding-examples` includes a Devnet IDL
copy at the same program ID with version `1.5.5` and the additional
`validate_stat_v2` instruction. This SDK implements the PR parity surface while
remaining Devnet-only.

## Canonical Values

| Value | Devnet |
| --- | --- |
| Solana RPC | `https://api.devnet.solana.com` |
| API base | `https://txline-dev.txodds.com/api` |
| Guest JWT | `https://txline-dev.txodds.com/auth/guest/start` |
| Program ID | `6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J` |
| TxL mint | `4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG` |
| USDT mint | `ELWTKspHKCnCfCiCiqYw1EDH77k8VCP74dK9qytG2Ujh` |

## Guardrails

- `Network` has only `Devnet`.
- There is no `mainnet` feature.
- Config validation rejects mixed non-RPC values.
- Empty RPC URLs are rejected.
- Obvious mainnet-looking RPC URLs are rejected.
- Program ID and mints stay fixed when `with_rpc_url()` is used.
- Examples require explicit Devnet environment variables and contain no
  secrets.
- `seq` is rejected when it is zero or negative before validation requests are
  sent.
- Default tests do not contact live Devnet RPC or TxLINE APIs.

## Custom RPC URLs

`with_rpc_url()` exists for custom Devnet RPC providers. A syntactic check can
catch accidental URLs containing clear mainnet markers, but it cannot prove that
an arbitrary provider is actually connected to Devnet.

Callers must provide a Devnet RPC endpoint whenever overriding the default RPC.

```rust,no_run
# use txline::TxlineConfig;
let cfg = TxlineConfig::devnet()
    .with_rpc_url("https://custom-rpc.example.com/solana/devnet");
```

## Live Validation

Normal tests do not require live credentials. Examples that contact Devnet need
caller-provided values such as:

```bash
TXLINE_DEVNET_JWT=...
TXLINE_DEVNET_API_TOKEN=...
TXLINE_FIXTURE_ID=17952170
TXLINE_SCORE_SEQ=941
TXLINE_WALLET=/path/to/devnet-wallet.json
TXLINE_VALIDATE_ON_CHAIN=1
```

Do not fake live validation. If credentials are unavailable, report the flow as
not run.

## Free Tier Notes

Free tiers do not require TxL payment, but users still need Devnet SOL for
Solana fees and possible rent.

`devnet_free_tier` and `devnet_setup_user` can run the setup flow from a wallet
path. They fetch pricing matrix data, create the user's Token-2022 ATA when
missing, submit `subscribe`, sign the activation message, and keep the activated
API token in memory. Do not log or commit tokens, wallet signatures, keypairs,
seed phrases, or private keys.
