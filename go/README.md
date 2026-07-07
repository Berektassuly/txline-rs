# TxLINE Go SDK

Backend-friendly Go SDK for TxLINE Devnet REST, SSE, validation payloads, Solana instruction builders, and purchase quote safety checks.

This module is Devnet-first. Mainnet addresses are documented in public TxLINE docs, but this SDK does not present mainnet transaction flows as production-ready parity.

## Install

Published package docs: <https://pkg.go.dev/github.com/Berektassuly/txline/go/txline>

Add the SDK to a Go module:

```bash
go get github.com/Berektassuly/txline/go/txline@v0.4.0
```

Develop from this repository:

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

## World Cup Trading Lifecycle

The Go SDK includes a Devnet-only lifecycle layer for World Cup-style score markets. It is intentionally SDK-side orchestration over documented TxLINE data APIs and public Devnet IDL builders; it does not create unpublished trading REST clients or derive trading PDAs whose seeds are not published.

Useful references:

- World Cup hackathon: <https://superteam.fun/earn/hackathon/world-cup/>
- On-chain validation: <https://txline.txodds.com/documentation/examples/onchain-validation>
- Streaming data: <https://txline.txodds.com/documentation/examples/streaming-data>
- Devnet IDL JSON: <https://github.com/txodds/tx-on-chain/blob/main/examples/devnet/idl/txoracle.json>

Lifecycle outline:

1. Start a guest session, complete Devnet `subscribe`, sign the activation preimage, and set both guest JWT and activated API token.
2. Define score market terms with `FinalOutcomeMarketTerms`, `TotalGoalsMarketTerms`, `SpreadMarketTerms`, or an explicit `ScoreMarketTerms`.
3. Pass an application-owned 32-byte terms hash into `CreateIntentPlan` or `CreateTradePlan`. The SDK does not derive this hash because the production preimage is not documented in the public Devnet materials.
4. Build matching, close, settlement, claim, refund, and audit plans with explicit caller-supplied accounts.
5. Observe live scores or odds with `Scores().StreamFixture`, `Scores().HistoricalByFixture`, `Odds().StreamFixture`, or the corresponding snapshot/update clients.
6. Detect settlement with `IsFinalOutcomeRecord` and `ExtractFinalOutcome`.
7. Fetch V2 score proof payloads with `Scores().StatValidationV2` using `FinalOutcomeStatKeys`.
8. Build validation and settlement instructions with `NewFinalOutcomeProof`, `ValidateStatV2Plan`, `SettleTradePlan`, `SettleMatchedTradePlan`, or `AuditTradeResultPlan`.
9. Use `ClaimViaResolutionPlan`, `ClaimBatchLegacyPlan`, or `RefundBatchPlan` only when the application has the required public Devnet accounts and real resolution proof material.

Final-outcome proof assembly:

```go
ctx := context.Background()
client, err := txline.NewClient(txline.DevnetConfig())
if err != nil {
    return err
}

// Set a guest JWT and activated API token obtained through the Devnet
// subscription flow before calling data endpoints.
client.SetGuestJWT(guestJWT)
client.SetAPIToken(apiToken)

fixtureID := int64(17952170)
scores, err := client.Scores().HistoricalByFixture(ctx, fixtureID)
if err != nil {
    return err
}

cfg := txline.DefaultSoccerFinalOutcomeConfig()
outcome, err := txline.FindFinalOutcome(scores, cfg)
if err != nil {
    return err
}

validation, err := client.Scores().StatValidationV2(
    ctx,
    outcome.FixtureID,
    outcome.Seq,
    txline.FinalOutcomeStatKeys(outcome.Config),
)
if err != nil {
    return err
}

proof, err := txline.NewFinalOutcomeProof(outcome, validation)
if err != nil {
    return err
}

validatePlan, err := txline.ValidateStatV2Plan(proof.Payload, proof.Strategy)
if err != nil {
    return err
}
_ = validatePlan.Instructions
```

Intent and settlement planning:

```go
terms, err := txline.FinalOutcomeMarketTerms(
    fixtureID,
    txline.MarketSideParticipant1,
    txline.DefaultSoccerFinalOutcomeConfig(),
)
if err != nil {
    return err
}

// The coordinating application or backend must define this hash preimage and
// pass the resulting 32 bytes into Devnet create_intent/create_trade flows.
var termsHash [32]byte

intentPlan, err := txline.CreateIntentPlan(txline.CreateIntentPlanParams{
    Accounts:      createIntentAccounts,
    Terms:         terms,
    TermsHash:     termsHash,
    IntentID:      intentID,
    DepositAmount: depositAmount,
    ExpirationTS:  expirationTS,
    ClaimPeriod:   claimPeriod,
})
if err != nil {
    return err
}
_ = intentPlan.Instructions

settlePlan, err := txline.SettleMatchedTradePlan(txline.SettleMatchedTradePlanParams{
    Accounts: settleMatchedTradeAccounts,
    TradeID:  tradeID,
    Terms:    terms,
    Payload:  proof.Payload,
})
if err != nil {
    return err
}
_ = settlePlan.Instructions
```

Limitations:

- Runtime support remains Devnet-only.
- Trading plans require caller-supplied accounts, signatures, market IDs, token accounts, vaults, and off-chain coordination.
- Terms hashes are explicit inputs. The SDK preserves them but does not invent a production hash preimage.
- Settlement helpers require real score records and proof payloads from TxLINE APIs. Default tests stay offline and do not simulate live Devnet transactions.

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
