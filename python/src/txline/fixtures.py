"""Fixture REST DTOs and clients."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from txline.errors import InvalidInputError
from txline.validation.legacy import UpdateStats
from txline.validation.proof import Hash32, ProofNode, proof_nodes_from_json

FIXTURE_GAME_STATE_SCHEDULED = 1
FIXTURE_GAME_STATE_CANCELLED = 6


@dataclass(frozen=True, slots=True)
class BatchMetadata:
    total_update_count: int
    num_unique_fixtures: int
    overall_batch_start_ts: int
    overall_batch_end_ts: int

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> BatchMetadata:
        return cls(
            total_update_count=int(data["totalUpdateCount"]),
            num_unique_fixtures=int(data["numUniqueFixtures"]),
            overall_batch_start_ts=int(data["overallBatchStartTs"]),
            overall_batch_end_ts=int(data["overallBatchEndTs"]),
        )


@dataclass(frozen=True, slots=True)
class Fixture:
    ts: int
    start_time: int
    competition: str
    competition_id: int
    fixture_group_id: int
    participant1_id: int
    participant1: str
    participant2_id: int
    participant2: str
    fixture_id: int
    participant1_is_home: bool
    game_state: int | None = None
    extra: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Fixture:
        known = {
            "Ts",
            "StartTime",
            "Competition",
            "CompetitionId",
            "FixtureGroupId",
            "Participant1Id",
            "Participant1",
            "Participant2Id",
            "Participant2",
            "FixtureId",
            "Participant1IsHome",
            "GameState",
        }
        return cls(
            ts=int(data["Ts"]),
            start_time=int(data["StartTime"]),
            competition=str(data["Competition"]),
            competition_id=int(data["CompetitionId"]),
            fixture_group_id=int(data["FixtureGroupId"]),
            participant1_id=int(data["Participant1Id"]),
            participant1=str(data["Participant1"]),
            participant2_id=int(data["Participant2Id"]),
            participant2=str(data["Participant2"]),
            fixture_id=int(data["FixtureId"]),
            participant1_is_home=bool(data["Participant1IsHome"]),
            game_state=int(data["GameState"]) if data.get("GameState") is not None else None,
            extra={key: value for key, value in data.items() if key not in known},
        )

    def is_scheduled(self) -> bool:
        return self.game_state == FIXTURE_GAME_STATE_SCHEDULED

    def is_cancelled(self) -> bool:
        return self.game_state == FIXTURE_GAME_STATE_CANCELLED


@dataclass(frozen=True, slots=True)
class FixtureBatchSummary:
    fixture_id: int
    competition_id: int
    competition: str
    update_stats: UpdateStats
    update_sub_tree_root: Hash32

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> FixtureBatchSummary:
        return cls(
            fixture_id=int(data["fixtureId"]),
            competition_id=int(data["competitionId"]),
            competition=str(data["competition"]),
            update_stats=UpdateStats.from_dict(data["updateStats"]),
            update_sub_tree_root=Hash32.decode(data["updateSubTreeRoot"]),
        )


@dataclass(frozen=True, slots=True)
class FixtureValidation:
    snapshot: Fixture
    summary: FixtureBatchSummary
    sub_tree_proof: list[ProofNode]
    main_tree_proof: list[ProofNode]

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> FixtureValidation:
        return cls(
            snapshot=Fixture.from_dict(data["snapshot"]),
            summary=FixtureBatchSummary.from_dict(data["summary"]),
            sub_tree_proof=proof_nodes_from_json(data.get("subTreeProof")),
            main_tree_proof=proof_nodes_from_json(data.get("mainTreeProof")),
        )


@dataclass(frozen=True, slots=True)
class FixtureBatchValidation:
    metadata: BatchMetadata
    proof: list[ProofNode]

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> FixtureBatchValidation:
        return cls(
            metadata=BatchMetadata.from_dict(data["metadata"]),
            proof=proof_nodes_from_json(data.get("proof")),
        )


class FixturesClient:
    def __init__(self, client: Any) -> None:
        self._client = client

    def snapshot(
        self, start_epoch_day: int | None = None, competition_id: int | None = None
    ) -> list[Fixture]:
        query: list[tuple[str, str]] = []
        if start_epoch_day is not None:
            query.append(("startEpochDay", str(start_epoch_day)))
        if competition_id is not None:
            query.append(("competitionId", str(competition_id)))
        return [
            Fixture.from_dict(item)
            for item in self._client._get_json("/fixtures/snapshot", query, True)
        ]

    def updates(self, epoch_day: int, hour_of_day: int) -> list[Fixture]:
        validate_hour(hour_of_day)
        return [
            Fixture.from_dict(item)
            for item in self._client._get_json(
                f"/fixtures/updates/{epoch_day}/{hour_of_day}", [], True
            )
        ]

    def validation(self, fixture_id: int, timestamp: int | None = None) -> FixtureValidation:
        query = [("fixtureId", str(fixture_id))]
        if timestamp is not None:
            query.append(("timestamp", str(timestamp)))
        return FixtureValidation.from_dict(
            self._client._get_json("/fixtures/validation", query, True)
        )

    def batch_validation(self, epoch_day: int, hour_of_day: int) -> FixtureBatchValidation:
        validate_hour(hour_of_day)
        return FixtureBatchValidation.from_dict(
            self._client._get_json(
                "/fixtures/batch-validation",
                [("epochDay", str(epoch_day)), ("hourOfDay", str(hour_of_day))],
                True,
            )
        )


class AsyncFixturesClient:
    def __init__(self, client: Any) -> None:
        self._client = client

    async def snapshot(
        self, start_epoch_day: int | None = None, competition_id: int | None = None
    ) -> list[Fixture]:
        query: list[tuple[str, str]] = []
        if start_epoch_day is not None:
            query.append(("startEpochDay", str(start_epoch_day)))
        if competition_id is not None:
            query.append(("competitionId", str(competition_id)))
        data = await self._client._get_json("/fixtures/snapshot", query, True)
        return [Fixture.from_dict(item) for item in data]

    async def updates(self, epoch_day: int, hour_of_day: int) -> list[Fixture]:
        validate_hour(hour_of_day)
        data = await self._client._get_json(
            f"/fixtures/updates/{epoch_day}/{hour_of_day}", [], True
        )
        return [Fixture.from_dict(item) for item in data]

    async def validation(self, fixture_id: int, timestamp: int | None = None) -> FixtureValidation:
        query = [("fixtureId", str(fixture_id))]
        if timestamp is not None:
            query.append(("timestamp", str(timestamp)))
        data = await self._client._get_json("/fixtures/validation", query, True)
        return FixtureValidation.from_dict(data)

    async def batch_validation(self, epoch_day: int, hour_of_day: int) -> FixtureBatchValidation:
        validate_hour(hour_of_day)
        data = await self._client._get_json(
            "/fixtures/batch-validation",
            [("epochDay", str(epoch_day)), ("hourOfDay", str(hour_of_day))],
            True,
        )
        return FixtureBatchValidation.from_dict(data)


def validate_hour(hour_of_day: int) -> None:
    if hour_of_day < 0 or hour_of_day > 23:
        raise InvalidInputError("hour_of_day must be 0..=23")


def validate_interval(interval: int) -> None:
    if interval < 0 or interval > 11:
        raise InvalidInputError("interval must be the 0-indexed 5-minute bucket 0..=11")
