"""Odds REST DTOs and clients."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from txline.fixtures import validate_hour, validate_interval
from txline.validation.legacy import UpdateStats
from txline.validation.proof import Hash32, ProofNode, proof_nodes_from_json


@dataclass(frozen=True, slots=True)
class OddsPayload:
    fixture_id: int
    message_id: str
    ts: int
    bookmaker: str
    bookmaker_id: int
    super_odds_type: str
    game_state: str | None
    in_running: bool
    market_parameters: str | None
    market_period: str | None
    price_names: list[str]
    prices: list[int]
    pct: list[str]
    extra: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> OddsPayload:
        known = {
            "FixtureId",
            "MessageId",
            "Ts",
            "Bookmaker",
            "BookmakerId",
            "SuperOddsType",
            "GameState",
            "InRunning",
            "MarketParameters",
            "MarketPeriod",
            "PriceNames",
            "Prices",
            "Pct",
        }
        return cls(
            fixture_id=int(data["FixtureId"]),
            message_id=str(data["MessageId"]),
            ts=int(data["Ts"]),
            bookmaker=str(data["Bookmaker"]),
            bookmaker_id=int(data["BookmakerId"]),
            super_odds_type=str(data["SuperOddsType"]),
            game_state=str(data["GameState"]) if data.get("GameState") is not None else None,
            in_running=bool(data["InRunning"]),
            market_parameters=(
                str(data["MarketParameters"]) if data.get("MarketParameters") is not None else None
            ),
            market_period=str(data["MarketPeriod"])
            if data.get("MarketPeriod") is not None
            else None,
            price_names=[str(value) for value in data.get("PriceNames", [])],
            prices=[int(value) for value in data.get("Prices", [])],
            pct=[str(value) for value in data.get("Pct", [])],
            extra={key: value for key, value in data.items() if key not in known},
        )


@dataclass(frozen=True, slots=True)
class OddsBatchSummary:
    fixture_id: int
    update_stats: UpdateStats
    odds_sub_tree_root: Hash32

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> OddsBatchSummary:
        return cls(
            fixture_id=int(data["fixtureId"]),
            update_stats=UpdateStats.from_dict(data["updateStats"]),
            odds_sub_tree_root=Hash32.decode(data["oddsSubTreeRoot"]),
        )


@dataclass(frozen=True, slots=True)
class OddsValidation:
    odds: OddsPayload
    summary: OddsBatchSummary
    sub_tree_proof: list[ProofNode]
    main_tree_proof: list[ProofNode]

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> OddsValidation:
        return cls(
            odds=OddsPayload.from_dict(data["odds"]),
            summary=OddsBatchSummary.from_dict(data["summary"]),
            sub_tree_proof=proof_nodes_from_json(data.get("subTreeProof")),
            main_tree_proof=proof_nodes_from_json(data.get("mainTreeProof")),
        )


class OddsClient:
    def __init__(self, client: Any) -> None:
        self._client = client

    def snapshot(self, fixture_id: int, as_of: int | None = None) -> list[OddsPayload]:
        query = [("asOf", str(as_of))] if as_of is not None else []
        data = self._client._get_json(f"/odds/snapshot/{fixture_id}", query, True)
        return [OddsPayload.from_dict(item) for item in data]

    def live_updates_by_fixture(self, fixture_id: int) -> list[OddsPayload]:
        data = self._client._get_json(f"/odds/updates/{fixture_id}", [], True)
        return [OddsPayload.from_dict(item) for item in data]

    def historical_updates(
        self, epoch_day: int, hour_of_day: int, interval: int, fixture_id: int | None = None
    ) -> list[OddsPayload]:
        validate_hour(hour_of_day)
        validate_interval(interval)
        query = [("fixtureId", str(fixture_id))] if fixture_id is not None else []
        data = self._client._get_json(
            f"/odds/updates/{epoch_day}/{hour_of_day}/{interval}", query, True
        )
        return [OddsPayload.from_dict(item) for item in data]

    def validation(self, message_id: str, ts: int) -> OddsValidation:
        data = self._client._get_json(
            "/odds/validation", [("messageId", message_id), ("ts", str(ts))], True
        )
        return OddsValidation.from_dict(data)


class AsyncOddsClient:
    def __init__(self, client: Any) -> None:
        self._client = client

    async def snapshot(self, fixture_id: int, as_of: int | None = None) -> list[OddsPayload]:
        query = [("asOf", str(as_of))] if as_of is not None else []
        data = await self._client._get_json(f"/odds/snapshot/{fixture_id}", query, True)
        return [OddsPayload.from_dict(item) for item in data]

    async def live_updates_by_fixture(self, fixture_id: int) -> list[OddsPayload]:
        data = await self._client._get_json(f"/odds/updates/{fixture_id}", [], True)
        return [OddsPayload.from_dict(item) for item in data]

    async def historical_updates(
        self, epoch_day: int, hour_of_day: int, interval: int, fixture_id: int | None = None
    ) -> list[OddsPayload]:
        validate_hour(hour_of_day)
        validate_interval(interval)
        query = [("fixtureId", str(fixture_id))] if fixture_id is not None else []
        data = await self._client._get_json(
            f"/odds/updates/{epoch_day}/{hour_of_day}/{interval}", query, True
        )
        return [OddsPayload.from_dict(item) for item in data]

    async def validation(self, message_id: str, ts: int) -> OddsValidation:
        data = await self._client._get_json(
            "/odds/validation", [("messageId", message_id), ("ts", str(ts))], True
        )
        return OddsValidation.from_dict(data)
