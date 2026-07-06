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
