# TxLINE Python SDK

Devnet-first Python SDK for TxLINE REST APIs, SSE streams, validation helpers,
Solana instruction builders, and checked purchase quote transaction bytes.

> This package intentionally supports TxLINE Devnet only. Mainnet constants are
> documented in `NOTES.md` for source comparison, but mainnet transaction flows
> are not implemented or presented as safe.

## Development

```bash
cd python
python -m pip install -e ".[dev]"
python -m pytest
python -m ruff check .
python -m ruff format --check .
python -m mypy src
python -m build
```

Import name:

```python
import txline
```

## Quick Start

```python
from txline import ApiToken, GuestJwt, TxlineClient, TxlineConfig

with TxlineClient(TxlineConfig.devnet()) as client:
    guest = client.start_guest_session()
    client.set_guest_jwt(guest)
    client.set_api_token(ApiToken("caller-provided-activated-api-token"))

    fixtures = client.fixtures().snapshot()
    print(len(fixtures))
```

Async clients are available too:

```python
from txline import ApiToken, AsyncTxlineClient, TxlineConfig

async with AsyncTxlineClient(TxlineConfig.devnet()) as client:
    guest = await client.start_guest_session()
    client.set_guest_jwt(guest)
    client.set_api_token(ApiToken("caller-provided-activated-api-token"))
    scores = await client.scores().historical_by_fixture(17952170)
```

## REST Examples

```python
client.set_guest_jwt(GuestJwt("caller-provided-guest-jwt"))
client.set_api_token(ApiToken("caller-provided-api-token"))

odds = client.odds().snapshot(17952170)
scores = client.scores().snapshot(17952170)
legacy = client.scores().stat_validation_legacy(17952170, seq=941, stat_key=1002)
v2 = client.scores().stat_validation_v2(17952170, seq=941, stat_keys=[1001, 1002])
```

Activation preimage helper:

```python
message = client.activation_preimage("SUBSCRIBE_TX_SIGNATURE", [])
```

For an empty league bundle, this signs:

```text
SUBSCRIBE_TX_SIGNATURE::guest-jwt
```

## SSE Streams

Streams require both the guest JWT and activated API token to be set on the
client.

```python
from txline.sse import StreamOptions

async for event in client.scores_stream().stream(
    StreamOptions(fixture_id=17952170, initial_backoff=1.0, max_backoff=30.0)
):
    print(event.id, event.data.seq)
```

Streams parse SSE blocks, filter `event: heartbeat`, preserve
`Last-Event-ID`, respect `retry:` hints, reconnect after interruptions, and
refresh guest JWTs on stream `401`/`403`.

## Validation Helpers

```python
from txline.validation import Comparison, NDimensionalStrategy, TraderPredicate

strategy = (
    NDimensionalStrategy.builder(stat_count=2)
    .single(0, TraderPredicate(1, Comparison.GREATER_THAN))
    .build()
)

payload = v2.to_validation_input()
```

Proof hashes decode from base64, URL-safe base64, hex, or byte arrays and must
be exactly 32 bytes.

## Hackathon Trading Lifecycle

For the [World Cup hackathon](https://superteam.fun/earn/hackathon/world-cup/),
the Python SDK includes `txline.trading_lifecycle` helpers for score-based
prediction-market demos. The helpers compose the documented TxLINE data APIs,
V2 score-stat validation payloads, and public Devnet trading instruction
builders. They do not invent a trading REST API, derive unpublished trading
PDAs, manage wallets, or submit transactions.

Supported flow:

1. Subscribe and authenticate with `TxlineClient`, a guest JWT, and an activated
   API token.
2. Define market terms with `final_outcome_market_terms`,
   `total_goals_market_terms`, `spread_market_terms`, or `ScoreMarketTerms`.
3. Build intent, direct-trade, match, close, settlement, claim, refund, and
   audit plans with the plan helpers. Each plan returns ordered Solana
   instructions plus caller-owned account/signature boundaries.
4. Observe live odds or scores through REST or SSE streams. The streaming guide
   is at <https://txline.txodds.com/documentation/examples/streaming-data>.
5. Detect final-outcome score records using `action=game_finalised`,
   `statusId=100`, and `period=100`.
6. Fetch V2 stat-validation proof payloads and build validation or settlement
   instructions. The on-chain validation guide is at
   <https://txline.txodds.com/documentation/examples/onchain-validation>.
7. Use claim, refund, and audit plan helpers only when the caller has the
   required resolution roots, Merkle proofs, accounts, and signatures.

Final-outcome validation example:

```python
from txline import ApiToken, GuestJwt, TxlineClient, TxlineConfig
from txline.solana.instructions import validate_stat_v2_instruction
from txline.trading_lifecycle import (
    default_soccer_final_outcome_config,
    extract_final_outcome,
    final_outcome_stat_keys,
    final_outcome_strategy,
    is_final_outcome_record,
)
from txline.validation import timestamp_ms_to_epoch_day

fixture_id = 17952170

with TxlineClient(TxlineConfig.devnet()) as client:
    client.set_guest_jwt(GuestJwt("caller-provided-guest-jwt"))
    client.set_api_token(ApiToken("caller-provided-api-token"))

    scores = client.scores().historical_by_fixture(fixture_id)
    final_score = next(score for score in scores if is_final_outcome_record(score))
    outcome = extract_final_outcome(final_score, default_soccer_final_outcome_config())

    validation = client.scores().stat_validation_v2(
        outcome.fixture_id,
        outcome.seq,
        final_outcome_stat_keys(outcome.config),
    )

    payload = validation.to_validation_input()
    strategy = final_outcome_strategy(outcome)
    solana = client.solana()
    daily_scores = solana.pdas().daily_scores_roots(
        timestamp_ms_to_epoch_day(payload.fixture_summary.min_timestamp)
    )
    ix = validate_stat_v2_instruction(
        solana.program_id(),
        daily_scores.address,
        payload,
        strategy,
    )
```

Settlement plan helpers use the same V2 proof payload, but the caller still
supplies all trading accounts:

```python
from txline.trading_lifecycle import (
    final_outcome_market_terms,
    settle_matched_trade_plan,
    validation_input_for_market,
)

terms = final_outcome_market_terms(outcome.fixture_id, outcome.side, outcome.config)
settlement_payload = validation_input_for_market(validation, terms)
plan = settle_matched_trade_plan(
    solana.program_id(),
    caller_supplied_settle_matched_trade_accounts,
    trade_id=caller_supplied_trade_id,
    validation_input=settlement_payload,
    terms=terms,
)
```

The Devnet IDL requires 32-byte `terms_hash` values for intent, trade, and
legacy claim flows. The public docs and IDL do not define a production preimage
format, so the SDK validates caller-provided bytes and does not derive hashes
from ad hoc strings. The public Devnet IDL is available at
<https://github.com/txodds/tx-on-chain/blob/main/examples/devnet/idl/txoracle.json>.

## Solana Instruction Builders

```python
from txline.solana import Pubkey
from txline.solana.instructions import subscribe_instruction

solana = client.solana()
pdas = solana.pdas()
user = Pubkey.from_string("11111111111111111111111111111111")

ix = subscribe_instruction(
    solana.program_id(),
    pdas.subscribe_accounts(user),
    service_level_id=1,
    weeks=4,
)
```

The SDK builds deterministic instruction data and account metas for
subscription, faucet, purchase, validation, and low-level public trading
instructions. It does not sign or send transactions.

## Purchase Quote Safety

Use checked purchase quote flows before signing backend-provided transaction
bytes:

```python
checked = client.purchase_quote_checked(
    buyer="buyer-public-key",
    txline_amount=1_000,
    expected_backend_signer="backend-public-key",
)
tx_bytes = checked.transaction_bytes()
```

The checker verifies the expected backend signer, decodes the transaction,
rejects address table lookups, limits invoked programs, requires exactly one
TxLINE `purchase_subscription_token_usdt` instruction, verifies the requested
amount, and checks the expected Devnet account layout.

Live examples require caller-provided credentials, wallet signatures, and
wallet/RPC code. Do not commit JWTs, API tokens, private keys, seed phrases,
wallet signatures, or full auth headers.
