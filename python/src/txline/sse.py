"""Server-Sent Events parsing and resilient stream iterators."""

from __future__ import annotations

import asyncio
import contextlib
import json
import time
from collections.abc import AsyncIterator, Callable, Iterator
from dataclasses import dataclass
from typing import Any, Generic, TypeVar

from txline.errors import HttpStatusError
from txline.odds import OddsPayload
from txline.scores import Scores

T = TypeVar("T")


@dataclass(frozen=True, slots=True)
class RawSseEvent:
    id: str | None = None
    event: str | None = None
    data: str = ""
    retry: int | None = None


@dataclass(frozen=True, slots=True)
class SseEvent(Generic[T]):
    id: str | None
    event: str | None
    data: T


@dataclass(frozen=True, slots=True)
class StreamOptions:
    fixture_id: int | None = None
    last_event_id: str | None = None
    initial_backoff: float = 1.0
    max_backoff: float = 30.0


class SseDecoder:
    def __init__(self) -> None:
        self._buffer = ""

    def push(self, chunk: bytes) -> list[RawSseEvent]:
        self._buffer += chunk.decode("utf-8")
        events: list[RawSseEvent] = []
        while True:
            split = _split_sse_block(self._buffer)
            if split is None:
                break
            block, self._buffer = split
            event = parse_sse_block(block)
            if event is not None:
                events.append(event)
        return events

    def finish(self) -> RawSseEvent | None:
        if not self._buffer.strip():
            self._buffer = ""
            return None
        event = parse_sse_block(self._buffer)
        self._buffer = ""
        return event


def parse_sse_block(block: str) -> RawSseEvent | None:
    event_id: str | None = None
    event_type: str | None = None
    retry: int | None = None
    data_lines: list[str] = []
    for raw_line in block.splitlines():
        if not raw_line or raw_line.startswith(":"):
            continue
        if ":" in raw_line:
            field, value = raw_line.split(":", 1)
            if value.startswith(" "):
                value = value[1:]
        else:
            field, value = raw_line, ""
        if field == "id":
            event_id = value
        elif field == "event":
            event_type = value
        elif field == "data":
            data_lines.append(value)
        elif field == "retry":
            with contextlib.suppress(ValueError):
                retry = int(value)
    data = "\n".join(data_lines)
    if event_id is None and event_type is None and not data:
        return None
    return RawSseEvent(id=event_id, event=event_type, data=data, retry=retry)


class AsyncSseStreamClient:
    def __init__(self, client: Any, path: str) -> None:
        self._client = client
        self._path = path

    def stream(self, options: StreamOptions | None = None) -> AsyncIterator[SseEvent[Any]]:
        parser = OddsPayload.from_dict if self._path == "/odds/stream" else Scores.from_dict
        return _async_typed_stream(self._client, self._path, options or StreamOptions(), parser)

    def stream_all(self) -> AsyncIterator[SseEvent[Any]]:
        return self.stream()

    def stream_fixture(self, fixture_id: int) -> AsyncIterator[SseEvent[Any]]:
        return self.stream(StreamOptions(fixture_id=fixture_id))


class SyncSseStreamClient:
    def __init__(self, client: Any, path: str) -> None:
        self._client = client
        self._path = path

    def stream(self, options: StreamOptions | None = None) -> Iterator[SseEvent[Any]]:
        parser = OddsPayload.from_dict if self._path == "/odds/stream" else Scores.from_dict
        return _sync_typed_stream(self._client, self._path, options or StreamOptions(), parser)

    def stream_all(self) -> Iterator[SseEvent[Any]]:
        return self.stream()

    def stream_fixture(self, fixture_id: int) -> Iterator[SseEvent[Any]]:
        return self.stream(StreamOptions(fixture_id=fixture_id))


async def _async_typed_stream(
    client: Any,
    path: str,
    options: StreamOptions,
    parser: Callable[[dict[str, Any]], T],
) -> AsyncIterator[SseEvent[T]]:
    last_event_id = options.last_event_id
    backoff = options.initial_backoff
    while True:
        query = [("fixtureId", str(options.fixture_id))] if options.fixture_id is not None else []
        headers = client.auth_headers(True).to_headers()
        headers["Accept"] = "text/event-stream"
        headers["Cache-Control"] = "no-cache"
        if last_event_id is not None:
            headers["Last-Event-ID"] = last_event_id
        async with client._http.stream(
            "GET", client._api_url(path), params=query, headers=headers
        ) as response:
            if response.status_code in {401, 403}:
                await client._refresh_guest_session_after_failure(client.guest_jwt())
                continue
            if not 200 <= response.status_code <= 299:
                body = await response.aread()
                raise HttpStatusError(response.status_code, body)
            backoff = options.initial_backoff
            decoder = SseDecoder()
            async for chunk in response.aiter_bytes():
                for raw_event in decoder.push(chunk):
                    if raw_event.id is not None:
                        last_event_id = raw_event.id
                    if raw_event.retry is not None:
                        backoff = min(raw_event.retry / 1000, options.max_backoff)
                    typed = _typed_event_from_raw(raw_event, parser)
                    if typed is not None:
                        yield typed
            tail = decoder.finish()
            if tail is not None:
                if tail.id is not None:
                    last_event_id = tail.id
                if tail.retry is not None:
                    backoff = min(tail.retry / 1000, options.max_backoff)
                typed = _typed_event_from_raw(tail, parser)
                if typed is not None:
                    yield typed
        await asyncio.sleep(backoff)
        backoff = min(backoff * 2, options.max_backoff)


def _sync_typed_stream(
    client: Any,
    path: str,
    options: StreamOptions,
    parser: Callable[[dict[str, Any]], T],
) -> Iterator[SseEvent[T]]:
    last_event_id = options.last_event_id
    backoff = options.initial_backoff
    while True:
        query = [("fixtureId", str(options.fixture_id))] if options.fixture_id is not None else []
        headers = client.auth_headers(True).to_headers()
        headers["Accept"] = "text/event-stream"
        headers["Cache-Control"] = "no-cache"
        if last_event_id is not None:
            headers["Last-Event-ID"] = last_event_id
        with client._http.stream(
            "GET", client._api_url(path), params=query, headers=headers
        ) as response:
            if response.status_code in {401, 403}:
                client._refresh_guest_session_after_failure(client.guest_jwt())
                continue
            if not 200 <= response.status_code <= 299:
                raise HttpStatusError(response.status_code, response.read())
            backoff = options.initial_backoff
            decoder = SseDecoder()
            for chunk in response.iter_bytes():
                for raw_event in decoder.push(chunk):
                    if raw_event.id is not None:
                        last_event_id = raw_event.id
                    if raw_event.retry is not None:
                        backoff = min(raw_event.retry / 1000, options.max_backoff)
                    typed = _typed_event_from_raw(raw_event, parser)
                    if typed is not None:
                        yield typed
            tail = decoder.finish()
            if tail is not None:
                if tail.id is not None:
                    last_event_id = tail.id
                if tail.retry is not None:
                    backoff = min(tail.retry / 1000, options.max_backoff)
                typed = _typed_event_from_raw(tail, parser)
                if typed is not None:
                    yield typed
        time.sleep(backoff)
        backoff = min(backoff * 2, options.max_backoff)


def _typed_event_from_raw(
    raw_event: RawSseEvent, parser: Callable[[dict[str, Any]], T]
) -> SseEvent[T] | None:
    if not raw_event.data or (raw_event.event or "").lower() == "heartbeat":
        return None
    data = parser(json.loads(raw_event.data))
    return SseEvent(id=raw_event.id, event=raw_event.event, data=data)


def _split_sse_block(buffer: str) -> tuple[str, str] | None:
    lf = buffer.find("\n\n")
    crlf = buffer.find("\r\n\r\n")
    if lf == -1 and crlf == -1:
        return None
    if crlf != -1 and (lf == -1 or crlf < lf):
        return buffer[:crlf], buffer[crlf + 4 :]
    return buffer[:lf], buffer[lf + 2 :]
