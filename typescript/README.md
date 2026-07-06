# TxLINE TypeScript SDK

TypeScript-first SDK for TxLINE Devnet REST, SSE, validation payload helpers, and Solana instruction builders.

This package is Devnet-first. Mainnet addresses are exposed only as verified reference data; transaction flows are not presented as Mainnet-ready.

## Install And Develop

```bash
npm install
npm run typecheck
npm test
npm run build
```

The package uses npm and a checked-in `package-lock.json` for deterministic Node-compatible installs.

## Quick Start

```ts
import { TxlineClient, devnetConfig } from "txline-sdk";

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

## Validation

```ts
import {
  comparison,
  strategyBuilder,
  traderPredicate,
  validateStatV2Instruction,
} from "txline-sdk";

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
} from "txline-sdk";

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
