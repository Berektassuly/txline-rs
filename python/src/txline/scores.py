"""Scores REST DTOs and clients."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from txline.errors import InvalidInputError
from txline.fixtures import validate_hour, validate_interval
from txline.validation.legacy import ScoresStatValidation, ensure_positive_seq
from txline.validation.v2 import ScoresStatValidationV2, ScoresStatValidationV2Response

SCORE_ACTION_GAME_FINALISED = "game_finalised"
FINAL_SETTLEMENT_STATUS_ID = 100
FINAL_SETTLEMENT_PERIOD = 100


@dataclass(frozen=True, slots=True)
class PlayerStats:
    goals: int | None = None
    own_goals: int | None = None
    penalty_attempts: int | None = None
    penalty_goals: int | None = None
    red_cards: int | None = None
    shots: int | None = None
    yellow_cards: int | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> PlayerStats:
        return cls(
            goals=_optional_int(data.get("goals")),
            own_goals=_optional_int(data.get("ownGoals")),
            penalty_attempts=_optional_int(data.get("penaltyAttempts")),
            penalty_goals=_optional_int(data.get("penaltyGoals")),
            red_cards=_optional_int(data.get("redCards")),
            shots=_optional_int(data.get("shots")),
            yellow_cards=_optional_int(data.get("yellowCards")),
        )


@dataclass(frozen=True, slots=True)
class PlayerStatsForParticipants:
    participant1: dict[int, PlayerStats]
    participant2: dict[int, PlayerStats]

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> PlayerStatsForParticipants:
        return cls(
            participant1={
                int(key): PlayerStats.from_dict(value)
                for key, value in data.get("Participant1", {}).items()
            },
            participant2={
                int(key): PlayerStats.from_dict(value)
                for key, value in data.get("Participant2", {}).items()
            },
        )


@dataclass(frozen=True, slots=True)
class Scores:
    fixture_id: int
    game_state: str
    start_time: int
    is_team: bool
    fixture_group_id: int
    competition_id: int
    country_id: int
    sport_id: int
    participant1_is_home: bool
    participant2_id: int
    participant1_id: int
    action: str
    id: int
    ts: int
    connection_id: int
    seq: int
    status_id: int | None = None
    period: int | None = None
    stats: dict[str, int] | None = None
    player_stats: PlayerStatsForParticipants | None = None
    extra: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Scores:
        known = {
            "fixtureId",
            "gameState",
            "startTime",
            "isTeam",
            "fixtureGroupId",
            "competitionId",
            "countryId",
            "sportId",
            "participant1IsHome",
            "participant2Id",
            "participant1Id",
            "action",
            "id",
            "ts",
            "connectionId",
            "seq",
            "statusId",
            "period",
            "stats",
            "PlayerStats",
        }
        return cls(
            fixture_id=int(data["fixtureId"]),
            game_state=str(data["gameState"]),
            start_time=int(data["startTime"]),
            is_team=bool(data["isTeam"]),
            fixture_group_id=int(data["fixtureGroupId"]),
            competition_id=int(data["competitionId"]),
            country_id=int(data["countryId"]),
            sport_id=int(data["sportId"]),
            participant1_is_home=bool(data["participant1IsHome"]),
            participant2_id=int(data["participant2Id"]),
            participant1_id=int(data["participant1Id"]),
            action=str(data["action"]),
            id=int(data["id"]),
            ts=int(data["ts"]),
            connection_id=int(data["connectionId"]),
            seq=int(data["seq"]),
            status_id=_optional_int(data.get("statusId")),
            period=_optional_int(data.get("period")),
            stats={str(key): int(value) for key, value in data.get("stats", {}).items()}
            if data.get("stats") is not None
            else None,
            player_stats=PlayerStatsForParticipants.from_dict(data["PlayerStats"])
            if data.get("PlayerStats") is not None
            else None,
            extra={key: value for key, value in data.items() if key not in known},
        )

    def is_final_outcome_record(self) -> bool:
        return (
            self.action == SCORE_ACTION_GAME_FINALISED
            and self.status_id == FINAL_SETTLEMENT_STATUS_ID
            and self.period == FINAL_SETTLEMENT_PERIOD
        )


class ScoresClient:
    def __init__(self, client: Any) -> None:
        self._client = client

    def snapshot(self, fixture_id: int, as_of: int | None = None) -> list[Scores]:
        query = [("asOf", str(as_of))] if as_of is not None else []
        data = self._client._get_json(f"/scores/snapshot/{fixture_id}", query, True)
        return [Scores.from_dict(item) for item in data]

    def live_updates_by_fixture(self, fixture_id: int) -> list[Scores]:
        data = self._client._get_json(f"/scores/updates/{fixture_id}", [], True)
        return [Scores.from_dict(item) for item in data]

    def historical_updates(
        self, epoch_day: int, hour_of_day: int, interval: int, fixture_id: int | None = None
    ) -> list[Scores]:
        validate_hour(hour_of_day)
        validate_interval(interval)
        query = [("fixtureId", str(fixture_id))] if fixture_id is not None else []
        data = self._client._get_json(
            f"/scores/updates/{epoch_day}/{hour_of_day}/{interval}", query, True
        )
        return [Scores.from_dict(item) for item in data]

    def historical_by_fixture(self, fixture_id: int) -> list[Scores]:
        data = self._client._get_json(f"/scores/historical/{fixture_id}", [], True)
        return [Scores.from_dict(item) for item in data]

    def stat_validation_legacy(
        self, fixture_id: int, seq: int, stat_key: int, stat_key2: int | None = None
    ) -> ScoresStatValidation:
        ensure_positive_seq(seq)
        query = [("fixtureId", str(fixture_id)), ("seq", str(seq)), ("statKey", str(stat_key))]
        if stat_key2 is not None:
            query.append(("statKey2", str(stat_key2)))
        return ScoresStatValidation.from_dict(
            self._client._get_json("/scores/stat-validation", query, True)
        )

    def stat_validation_v2(
        self, fixture_id: int, seq: int, stat_keys: list[int] | tuple[int, ...]
    ) -> ScoresStatValidationV2:
        ensure_positive_seq(seq)
        if not stat_keys:
            raise InvalidInputError("V2 stat validation requires at least one stat key")
        keys = [int(key) for key in stat_keys]
        data = self._client._get_json(
            "/scores/stat-validation",
            [
                ("fixtureId", str(fixture_id)),
                ("seq", str(seq)),
                ("statKeys", ",".join(str(key) for key in keys)),
            ],
            True,
        )
        return ScoresStatValidationV2.from_response(
            keys, ScoresStatValidationV2Response.from_dict(data)
        )


class AsyncScoresClient:
    def __init__(self, client: Any) -> None:
        self._client = client

    async def snapshot(self, fixture_id: int, as_of: int | None = None) -> list[Scores]:
        query = [("asOf", str(as_of))] if as_of is not None else []
        data = await self._client._get_json(f"/scores/snapshot/{fixture_id}", query, True)
        return [Scores.from_dict(item) for item in data]

    async def live_updates_by_fixture(self, fixture_id: int) -> list[Scores]:
        data = await self._client._get_json(f"/scores/updates/{fixture_id}", [], True)
        return [Scores.from_dict(item) for item in data]

    async def historical_updates(
        self, epoch_day: int, hour_of_day: int, interval: int, fixture_id: int | None = None
    ) -> list[Scores]:
        validate_hour(hour_of_day)
        validate_interval(interval)
        query = [("fixtureId", str(fixture_id))] if fixture_id is not None else []
        data = await self._client._get_json(
            f"/scores/updates/{epoch_day}/{hour_of_day}/{interval}", query, True
        )
        return [Scores.from_dict(item) for item in data]

    async def historical_by_fixture(self, fixture_id: int) -> list[Scores]:
        data = await self._client._get_json(f"/scores/historical/{fixture_id}", [], True)
        return [Scores.from_dict(item) for item in data]

    async def stat_validation_legacy(
        self, fixture_id: int, seq: int, stat_key: int, stat_key2: int | None = None
    ) -> ScoresStatValidation:
        ensure_positive_seq(seq)
        query = [("fixtureId", str(fixture_id)), ("seq", str(seq)), ("statKey", str(stat_key))]
        if stat_key2 is not None:
            query.append(("statKey2", str(stat_key2)))
        data = await self._client._get_json("/scores/stat-validation", query, True)
        return ScoresStatValidation.from_dict(data)

    async def stat_validation_v2(
        self, fixture_id: int, seq: int, stat_keys: list[int] | tuple[int, ...]
    ) -> ScoresStatValidationV2:
        ensure_positive_seq(seq)
        if not stat_keys:
            raise InvalidInputError("V2 stat validation requires at least one stat key")
        keys = [int(key) for key in stat_keys]
        data = await self._client._get_json(
            "/scores/stat-validation",
            [
                ("fixtureId", str(fixture_id)),
                ("seq", str(seq)),
                ("statKeys", ",".join(str(key) for key in keys)),
            ],
            True,
        )
        return ScoresStatValidationV2.from_response(
            keys, ScoresStatValidationV2Response.from_dict(data)
        )


def _optional_int(value: Any) -> int | None:
    return int(value) if value is not None else None
