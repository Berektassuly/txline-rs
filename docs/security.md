# Security

This document describes the SDK security boundaries. Use
[SECURITY.md](../SECURITY.md) for vulnerability reporting.

## Secrets

The SDK packages accept caller-provided guest JWTs, activated API tokens, wallet
signatures, and Solana signers. They do not manage private keys, seed phrases,
or durable secret storage.

Credential wrapper names vary by language, but `GuestJwt`/`GuestJWT`,
`ApiToken`/`APIToken`, and `AuthHeaders` redact formatted/debug output. Do not
log raw headers, request bodies, private keys, seed phrases, detached wallet
signatures, or complete tokens.

Credential constructors trim leading and trailing whitespace and reject empty or
whitespace-only values before those credentials can be placed in headers.

## Activation

The SDK centralizes the activation message:

```text
${txSig}:${selectedLeagues.join(",")}:${jwt}
```

Empty league lists produce:

```text
${txSig}::${jwt}
```

The wallet signature must come from the wallet that submitted the Devnet
`subscribe` transaction.

This compact activation message is compatibility-bound to the upstream API
contract. Callers should only sign it for the matching TxLINE Devnet host,
network, and subscription transaction they intend to activate.

## RPC Endpoints

Devnet config builders keep the TxLINE program ID and mints fixed to Devnet.
Validation rejects empty and obvious mainnet-looking RPC URLs, but
caller-provided custom RPC endpoints still need operator review.

Before using a custom provider, verify that it is connected to Solana Devnet and
that it is acceptable for the data, rate limits, and availability assumptions of
your application. A future live RPC genesis or version check could be added as
an explicit opt-in validation step, but the current guard is syntactic.

## Purchase Quotes

The SDK can request a Devnet purchase quote, decode the returned transaction
bytes, check the financial shape, and audit the transaction before the caller
signs it.

Use the checked purchase quote helper in your language package for signing or
submission flows. It fetches the quote, requires the expected backend signer,
validates the returned transaction, and only then exposes transaction bytes. Raw
or unchecked quote transaction accessors are diagnostics only and do not perform
safety validation.

The safety checker verifies:

- fee payer,
- required expected backend signer,
- invoked program IDs against the TxLINE purchase allowlist,
- exactly one TxLINE `purchase_subscription_token_usdt` instruction,
- instruction discriminator and requested TxL amount,
- Devnet mint, treasury, ATA, token program, system program, and associated
  token account metas,
- unexpected buyer signer usage.

Transactions that use address table lookups are rejected because dynamically
loaded accounts cannot be audited from the quote payload alone.

## Trading Builders

The public TxODDS trading builders are low-level instruction builders. They do
not validate mints, token programs, vault accounts, or PDA derivations, and they
do not prove that caller-supplied accounts match a specific market lifecycle.
Callers must review accounts, simulate when appropriate, and rely on deployed
on-chain constraints before signing or sending trading transactions.

## Streams

SSE clients send both credentials, preserve `Last-Event-ID`, and renew the guest
JWT on stream connection HTTP 401 and 403. Heartbeat events are filtered before
typed JSON deserialization.

REST requests refresh only on HTTP 401. REST 403 can mean entitlement failure,
an inactive API token, or a network mismatch, so the SDK does not silently
reinterpret it as an expired guest JWT.

Client instances that support cloning or sharing coalesce guest JWT refreshes.
Separate users should use separate client instances.

## Live Credentials

Default tests must not require real Devnet credentials. Live examples should be
run only when the required environment variables are present, and results should
state whether live validation actually ran.
