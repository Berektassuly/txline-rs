from __future__ import annotations

import asyncio

import httpx
import pytest

from txline import (
    API_TOKEN_HEADER,
    ApiToken,
    AsyncTxlineClient,
    GuestJwt,
    HttpStatusError,
    TxlineClient,
    TxlineConfig,
    activation_preimage,
)
from txline.errors import ConfigError, InvalidInputError
from txline.sse import SseDecoder, StreamOptions, parse_sse_block


def test_auth_wrapper_validation_and_redaction() -> None:
    jwt = GuestJwt(" guest.jwt ")
    token = ApiToken("\napi-token\t")
    assert jwt.as_str() == "guest.jwt"
    assert token.as_str() == "api-token"
    assert "guest.jwt" not in repr(jwt)
    assert "api-token" not in repr(token)
    with pytest.raises(InvalidInputError):
        GuestJwt(" \t\n")
    with pytest.raises(InvalidInputError):
        ApiToken("")


def test_activation_preimage_preserves_empty_league_slot() -> None:
    jwt = GuestJwt("jwt-value")
    assert activation_preimage("txSig", [], jwt) == "txSig::jwt-value"
    assert activation_preimage("txSig", [501, 804, 202], jwt) == "txSig:501,804,202:jwt-value"


def test_config_rpc_guardrails() -> None:
    cfg = TxlineConfig.devnet().with_rpc_url("https://custom-rpc.example.com/solana/devnet")
    cfg.validate()
    with pytest.raises(ConfigError, match="Devnet RPC endpoint"):
        TxlineConfig.devnet().with_rpc_url("https://api.mainnet-beta.solana.com").validate()
    with pytest.raises(ConfigError, match="must not be empty"):
        TxlineConfig.devnet().with_rpc_url(" ").validate()


def test_http_status_error_redacts_body_but_keeps_raw_bytes() -> None:
    err = HttpStatusError(500, b"secret-token")
    assert "secret-token" not in str(err)
    assert "redacted" in str(err)
    assert err.body == b"secret-token"


def test_sync_http_url_and_v2_statkeys_query_construction() -> None:
    seen: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        seen.append(request)
        return httpx.Response(200, json=_v2_response())

    client = TxlineClient(http_client=httpx.Client(transport=httpx.MockTransport(handler)))
    client.set_guest_jwt(GuestJwt("jwt"))
    client.set_api_token(ApiToken("api"))

    result = client.scores().stat_validation_v2(17952170, 941, [1001, 1002])

    assert result.requested_stat_keys == [1001, 1002]
    assert seen[0].url.path == "/api/scores/stat-validation"
    assert seen[0].url.params["fixtureId"] == "17952170"
    assert seen[0].url.params["seq"] == "941"
    assert seen[0].url.params["statKeys"] == "1001,1002"
    assert seen[0].headers[API_TOKEN_HEADER] == "api"


@pytest.mark.asyncio
async def test_async_sse_reconnect_refreshes_and_preserves_last_event_id() -> None:
    requests: list[httpx.Request] = []
    stream_calls = 0

    def handler(request: httpx.Request) -> httpx.Response:
        nonlocal stream_calls
        requests.append(request)
        if request.url.path == "/auth/guest/start":
            return httpx.Response(200, json={"token": "fresh"})
        if request.url.path == "/api/scores/stream":
            stream_calls += 1
            if stream_calls == 1:
                return httpx.Response(403, text="expired")
            if stream_calls == 2:
                return httpx.Response(
                    200,
                    content=b"id: 1\nevent: scores\ndata: " + _score_json(1).encode() + b"\n\n",
                    headers={"content-type": "text/event-stream"},
                )
            return httpx.Response(
                200,
                content=b"id: 2\nevent: scores\ndata: " + _score_json(2).encode() + b"\n\n",
                headers={"content-type": "text/event-stream"},
            )
        raise AssertionError(request.url)

    client = AsyncTxlineClient(
        http_client=httpx.AsyncClient(transport=httpx.MockTransport(handler))
    )
    client.set_guest_jwt(GuestJwt("stale"))
    client.set_api_token(ApiToken("api"))
    stream = client.scores_stream().stream(StreamOptions(initial_backoff=0.001, max_backoff=0.001))

    first = await anext(stream)
    second = await asyncio.wait_for(anext(stream), timeout=1)
    await stream.aclose()

    assert first.id == "1"
    assert second.id == "2"
    assert client.guest_jwt() == GuestJwt("fresh")
    third_stream_request = [req for req in requests if req.url.path == "/api/scores/stream"][2]
    assert third_stream_request.headers["Last-Event-ID"] == "1"
    await client.aclose()


def test_sse_parser_filters_heartbeat_and_handles_multiline_data() -> None:
    raw = parse_sse_block("id: a\nretry: 2500\nevent: stats\ndata: one\ndata: two")
    assert raw is not None
    assert raw.id == "a"
    assert raw.retry == 2500
    assert raw.data == "one\ntwo"

    decoder = SseDecoder()
    events = decoder.push(b'event: heartbeat\ndata: {"ok":true}\n\n')
    assert len(events) == 1
    assert events[0].event == "heartbeat"


def _v2_response() -> dict[str, object]:
    hash_bytes = [9] * 32
    return {
        "ts": 2,
        "statsToProve": [
            {"key": 1001, "value": 3, "period": 0},
            {"key": 1002, "value": 4, "period": 0},
        ],
        "eventStatRoot": hash_bytes,
        "summary": {
            "fixtureId": 17952170,
            "updateStats": {
                "updateCount": 1,
                "minTimestamp": 1,
                "maxTimestamp": 2,
            },
            "eventStatsSubTreeRoot": hash_bytes,
        },
        "statProofs": [[], []],
        "subTreeProof": [],
        "mainTreeProof": [],
    }


def _score_json(seq: int) -> str:
    return (
        '{"fixtureId":17952170,"gameState":"inprogress","startTime":1,"isTeam":true,'
        '"fixtureGroupId":1,"competitionId":2,"countryId":3,"sportId":4,'
        '"participant1IsHome":true,"participant2Id":20,"participant1Id":10,'
        f'"action":"score","id":99,"ts":2,"connectionId":77,"seq":{seq}}}'
    )
