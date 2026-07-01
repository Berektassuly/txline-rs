# Security

This SDK will touch credentials, wallet signatures, Solana transactions, and
settlement proofs. The scaffold does not implement those flows yet, but the
safety rules should shape every future change.

## Secrets

Never log or commit:

- guest JWTs,
- activated API tokens,
- private keys,
- seed phrases,
- unredacted `Authorization` headers,
- unredacted `X-Api-Token` headers,
- full request/response dumps that contain credentials.

Examples and tests should use placeholders or redacted prefixes only.

## Network Isolation

Mainnet and Devnet values must not be mixed. A valid on-chain transaction on one
network can still fail activation if sent to the other network's API host.

The SDK should eventually validate that these values come from the same network:

- Solana RPC URL,
- program ID,
- TxL mint,
- guest JWT URL,
- activation URL,
- API base URL,
- IDL or generated program type.

## Activation Signatures

The activation preimage is exact:

```text
${txSig}:${selectedLeagues.join(",")}:${jwt}
```

For empty leagues:

```text
${txSig}::${jwt}
```

The signature must be a base64 detached wallet signature from the wallet that
submitted the `subscribe` transaction.

## Purchase Quote Safety

Before signing a purchase quote transaction, future SDK code should inspect:

- fee payer equals the expected buyer wallet,
- backend/admin signature is present when required,
- every invoked program ID is allowed,
- buyer is not a signer for unexpected instructions,
- the decoded oracle instruction is the expected purchase instruction,
- the TxL amount matches the requested amount,
- there is exactly one expected oracle instruction.

Do not add transaction signing support until these checks exist.

## Error Handling

- Treat HTTP 401 as an expired or missing guest JWT. Renew the JWT from the same
  host and retry with the same API token.
- Treat HTTP 403 as invalid API token, insufficient entitlement, expired
  subscription, or network mismatch.
- Do not retry activation blindly after a signature error; rebuild and inspect
  the signed preimage first.
- Do not hide proof errors behind generic validation failures. Include
  non-secret context such as network, fixture ID, sequence, timestamp, and PDA.
