# TxLINE TypeScript SDK

TypeScript-first SDK for TxLINE Devnet REST, SSE, validation payload helpers, and Solana instruction builders.

This package is Devnet-first. Mainnet addresses are exposed only as verified reference data; transaction flows are not presented as Mainnet-ready.

## Install And Develop

Published package: <https://www.npmjs.com/package/@beriktassuly/txline>

Install from npm:

```bash
npm install @beriktassuly/txline
```

Develop from this repository:

```bash
npm install
npm run typecheck
npm test
npm run build
```

The package uses npm and a checked-in `package-lock.json` for deterministic Node-compatible installs.

## Quick Start

```ts
import { TxlineClient, devnetConfig } from "@beriktassuly/txline";

const client = new TxlineClient({ config: devnetConfig() });

const guestJwt = await client.startGuestSession();
client.setGuestJwt(guestJwt);

if (process.env.TXLINE_API_TOKEN) {
  client.setApiToken(process.env.TXLINE_API_TOKEN);
}

const fixtures = await client.fixtures().snapshot({ competitionId: 1 });
```

Live examples require caller-provided credentials, wallet signatures, and RPC/API access. Do not commit tokens, JWTs, signatures, seed phrases, private keys, generated live responses, or full auth headers.

## REST

```ts
const quote = await client.purchaseQuote(
  buyerAddress,
  1_000n,
);

const validation = await client.scores().statValidationV2({
  fixtureId: 17_952_170,
  seq: 1,
  statKeys: [1001, 1002],
});
```

REST methods build the current OpenAPI query names, including V2 score stat validation via `statKeys=1001,1002`.

## SSE

Streams require both the guest JWT and activated API token to be set on the
client.

```ts
const controller = new AbortController();

for await (const event of client.scoresStream().stream({
  fixtureId: 17_952_170,
  signal: controller.signal,
})) {
  console.log(event.id, event.data.seq);
}
```

The SSE client parses event blocks, filters `heartbeat`, preserves `Last-Event-ID`, respects server `retry:` hints, reconnects after interruptions, requires an activated API token, and refreshes guest JWTs on stream `401`/`403`.

## World Cup Trading Lifecycle

The SDK includes Devnet-only helpers for World Cup hackathon projects that need to connect TxLINE scores to score-settled trading or prediction-market flows. The helpers compose published REST, SSE, validation, and public Devnet IDL builders. They do not add unpublished trading REST endpoints, derive unverified trading PDAs, sign transactions, submit transactions, or compute a production terms-hash preimage.

Useful source links:

- [World Cup hackathon](https://superteam.fun/earn/hackathon/world-cup/)
- [On-chain validation guide](https://txline.txodds.com/documentation/examples/onchain-validation)
- [Streaming data guide](https://txline.txodds.com/documentation/examples/streaming-data)
- [Devnet IDL JSON](https://github.com/txodds/tx-on-chain/blob/main/examples/devnet/idl/txoracle.json)

Lifecycle outline:

1. Subscribe on Devnet and activate API access using `startGuestSession`, `subscribeInstruction`, `activationPreimage`, and `activateSubscription`, or inject caller-provided guest JWT and API token with `setGuestJwt` and `setApiToken`.
2. Define score-market terms with helpers such as `finalOutcomeMarketTerms`, `totalGoalsMarketTerms`, or `spreadMarketTerms`.
3. Pass an explicit 32-byte caller-owned terms hash to `createIntentPlan`, `createTradePlan`, or `claimBatchLegacyPlan`. The public Devnet IDL requires `[u8; 32]`, but the canonical production preimage is not published.
4. Observe live odds and scores through `oddsStream()` and `scoresStream()`, or fetch historical score records with `client.scores().historicalByFixture(fixtureId)`.
5. Detect final soccer outcomes with `isFinalOutcomeRecord` and `extractFinalOutcome`. The documented final-outcome record shape is `action=game_finalised`, `statusId=100`, and `period=100`; soccer defaults use stat key `1` for participant 1 goals and stat key `2` for participant 2 goals.
6. Fetch V2 proof payloads with `client.scores().statValidationV2({ fixtureId, seq, statKeys })`, then build validation inputs and strategies with `finalOutcomeProof`, `finalOutcomeStrategy`, `validationInputForMarket`, or `marketTermsStrategy`.
7. Build the on-chain instruction you need with `finalOutcomeValidationPlan`, `settleTradePlan`, `settleMatchedTradePlan`, `claimViaResolutionPlan`, `claimBatchLegacyPlan`, `refundBatchPlan`, or `auditTradeResultPlan`.

```ts
import {
  TxlineClient,
  defaultSoccerFinalOutcomeConfig,
  devnetConfig,
  extractFinalOutcome,
  finalOutcomeProof,
  finalOutcomeStatKeys,
  isFinalOutcomeRecord,
  validateStatV2Instruction,
} from "@beriktassuly/txline";

const client = new TxlineClient({ config: devnetConfig() });
client.setGuestJwt("caller-provided-guest-jwt");
client.setApiToken("caller-provided-api-token");

const fixtureId = 17_952_170;
const scores = await client.scores().historicalByFixture(fixtureId);
const finalScore = scores.find(isFinalOutcomeRecord);
if (!finalScore) throw new Error("final outcome not found");

const outcome = extractFinalOutcome(
  finalScore,
  defaultSoccerFinalOutcomeConfig(),
);

const validation = await client.scores().statValidationV2({
  fixtureId: outcome.fixtureId,
  seq: outcome.seq,
  statKeys: finalOutcomeStatKeys(outcome.config),
});

const proof = finalOutcomeProof(outcome, validation);
const dailyScoresRoot = "caller-provided-daily-scores-root";
const ix = validateStatV2Instruction(
  devnetConfig().programId,
  dailyScoresRoot,
  proof.payload,
  proof.strategy,
);
```

`dailyScoresRoot` is the Devnet daily scores root account for the proof timestamp. Applications may derive it with `DevnetPdas` or use `devnetFinalOutcomeValidationPlan` to derive it inside the SDK. Trading plans still require caller-supplied intent, escrow, vault, token, winner, resolver, and signer accounts from the coordinating application or backend.

## Validation

```ts
import {
  comparison,
  strategyBuilder,
  traderPredicate,
  validateStatV2Instruction,
} from "@beriktassuly/txline";

const strategy = strategyBuilder(validation.statsToProve().length)
  .single(0, traderPredicate(1, comparison.greaterThan()))
  .build();

const instruction = validateStatV2Instruction(
  devnetConfig().programId,
  dailyScoresRoot,
  validation.toValidationInput(),
  strategy,
);
```

Proof hashes may be base64, hex, or byte arrays and must decode to exactly 32 bytes. V2 helpers preserve requested stat-key order and reject mismatched `statsToProve`/`statProofs` lengths.

## Solana Builders

```ts
import {
  DEVNET_PROGRAM_ID,
  devnetSubscribeAccounts,
  subscribeInstruction,
} from "@beriktassuly/txline";

const accounts = await devnetSubscribeAccounts(userAddress);
const ix = subscribeInstruction(DEVNET_PROGRAM_ID, accounts, {
  serviceLevelId: 1,
  weeks: 4,
});
```

Trading builders are low-level by design: pass explicit accounts for `createIntent`, `createTrade`, `executeMatch`, settlement, claim, refund, and audit instructions. The SDK does not derive or validate unverified trading PDAs.

## Purchase Quote Safety

Backend quote transaction bytes are not considered safe for signing by default.

```ts
const checked = await client.purchaseQuoteChecked(
  buyerAddress,
  1_000n,
  expectedBackendSigner,
);

const bytes = checked.transactionBytes;
```

`purchaseQuoteChecked` validates financial shape, decodes the transaction, rejects address table lookups, checks allowed programs, requires exactly one TxLINE `purchase_subscription_token_usdt` instruction, verifies the discriminator and requested amount, checks the Devnet account layout, and requires the expected backend signer.

Raw helpers are intentionally named `Unchecked` for diagnostics only.

## Source Notes

See [NOTES.md](./NOTES.md) for public-source conflicts around OpenAPI version, public program docs, pinned Rust Devnet IDL coverage, and `validate_stat_v2`.
