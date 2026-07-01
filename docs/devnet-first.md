# Devnet First

The SDK should be developed on Devnet first and promoted to Mainnet only after
the same flow is proven against Mainnet constants.

## Why Devnet First

- It exercises the complete subscription and activation path without mainnet
  funds.
- It makes Token-2022 account setup, rent, and fee handling testable.
- It gives safe coverage for SSE reconnects and guest JWT renewal.
- It lets proof decoding and validation payloads mature before callers rely on
  mainnet settlement.

## Devnet Values

| Value | Devnet |
| --- | --- |
| Solana RPC | `https://api.devnet.solana.com` |
| API base | `https://txline-dev.txodds.com/api/` |
| Guest JWT | `https://txline-dev.txodds.com/auth/guest/start` |
| Program ID | `6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J` |
| TxL mint | `4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG` |

Free-tier subscriptions still require SOL for normal Solana fees and possible
rent. They do not require a TxL payment.

## Promotion Checklist

Before marking a feature mainnet-ready:

- Confirm the hosted docs and generated IDL for the selected network.
- Confirm the program ID, TxL mint, guest JWT host, API base, and activation URL
  are from the same network.
- Run the same flow on Devnet with a disposable wallet.
- Redact every credential from logs and test fixtures.
- Verify purchase quote transaction safety before adding signing support.
- Verify validation against a real score record sequence, not a placeholder.

## Mainnet V2 Note

Per the CTO update captured for this scaffold on 2026-07-01, Mainnet and Devnet
are now equivalent for the newest V2 score-validation flow. The new Mainnet
score proof behavior applies to score records from `2026-07-01 08:00 GMT`
onward.

Keep backward compatibility for older score proof formats. Users may still
validate historical score records that predate the new Mainnet behavior.
