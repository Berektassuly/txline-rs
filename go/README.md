# TxLINE Go SDK

Backend-friendly Go SDK for TxLINE Devnet REST, SSE, validation payloads, Solana instruction builders, and purchase quote safety checks.

This module is Devnet-first. Mainnet addresses are documented in public TxLINE docs, but this SDK does not present mainnet transaction flows as production-ready parity.

## Install

```bash
cd go
go mod tidy
go test ./...
go vet ./...
```

Import path:

```go
import txline "github.com/Berektassuly/txline/go/txline"
```

## Quick Start

```go
ctx := context.Background()

client, err := txline.NewClient(txline.DevnetConfig())
if err != nil {
    return err
}

guest, err := client.StartGuestSession(ctx)
if err != nil {
    return err
}
client.SetGuestJWT(guest.Token)

fixtures, err := client.Fixtures().Snapshot(ctx, nil, nil)
```

Live examples require caller-provided credentials, wallets, and signatures. Do not commit tokens, JWTs, private keys, seed phrases, wallet signatures, or full auth headers.

## REST Examples

```go
token, _ := txline.NewAPIToken(os.Getenv("TXLINE_API_TOKEN"))
client.SetAPIToken(token)

odds, err := client.Odds().Snapshot(ctx, 17952170, nil)
scores, err := client.Scores().HistoricalByFixture(ctx, 17952170)

validation, err := client.Scores().StatValidationV2(ctx, 17952170, 3, []uint32{1001, 1002})
```

All network methods accept `context.Context`. HTTP status and response bytes are preserved through `HTTPStatusError`; formatted errors redact response bodies.

## SSE Example

Streams require both the guest JWT and activated API token to be set on the
client.

```go
token, _ := txline.NewAPIToken(os.Getenv("TXLINE_API_TOKEN"))
client.SetAPIToken(token)

ctx, cancel := context.WithCancel(context.Background())
defer cancel()

stream := client.Scores().StreamFixture(ctx, 17952170)
for {
    select {
    case event, ok := <-stream.Events():
        if !ok {
            return nil
        }
        _ = event.Data
    case err := <-stream.Errors():
        _ = err
    }
}
```

The SSE client filters heartbeat events, preserves `Last-Event-ID`, honors server `retry:` hints, reconnects after interruption, refreshes guest JWTs on stream `401`/`403`, requires an activated API token, and stops on context cancellation.

## Validation Helpers

```go
hash, err := txline.DecodeHash32("0x...")

strategy, err := txline.NewStrategyBuilder(2).
    Single(0, txline.NewTraderPredicate(1, txline.GreaterThan())).
    Binary(0, 1, txline.Subtract(), txline.NewTraderPredicate(0, txline.EqualTo())).
    Build()
```

V2 score validation preserves requested `statKeys` order and rejects payloads whose `statsToProve` or `statProofs` lengths do not match.

## Solana Builders

```go
buyer, _ := txline.ParsePublicKey("...")
backend, _ := txline.ParsePublicKey("...")

accounts, err := txline.DevnetPurchaseSubscriptionTokenUSDTAccounts(buyer, backend)
ix, err := txline.PurchaseSubscriptionTokenUSDTInstruction(
    txline.DevnetProgramPublicKey(),
    accounts,
    1_000,
)
```

The SDK includes PDA helpers, Token-2022 ATA derivation, Token-2022 ATA creation, subscription/faucet/purchase builders, validation builders, and conservative low-level trading builders where callers supply explicit accounts.

## Purchase Quote Safety

```go
quote, err := client.PurchaseQuoteChecked(ctx, buyer, 1_000, backend)
if err != nil {
    return err
}
txBytes := quote.TransactionBytes()
```

Use `PurchaseQuoteChecked` or `ValidatedTransactionBytes` before signing or submitting quote transactions. Raw quote transaction bytes are exposed only through explicitly unchecked names.

The checker verifies the expected backend signer, safely decodes the transaction, rejects address lookup tables, allows only expected programs, requires exactly one TxLINE `purchase_subscription_token_usdt` instruction, checks the discriminator and requested amount, and validates the expected Devnet account layout.
