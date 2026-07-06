"""Legacy score stat validation DTOs."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from txline.errors import InvalidInputError, ValidationError
from txline.validation.proof import Hash32, ProofNode, proof_nodes_from_json


@dataclass(frozen=True, slots=True)
class ScoreStat:
    key: int
    value: int
    period: int

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ScoreStat:
        return cls(key=int(data["key"]), value=int(data["value"]), period=int(data["period"]))


@dataclass(frozen=True, slots=True)
class UpdateStats:
    update_count: int
    min_timestamp: int
    max_timestamp: int

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> UpdateStats:
        return cls(
            update_count=int(data["updateCount"]),
            min_timestamp=int(data["minTimestamp"]),
            max_timestamp=int(data["maxTimestamp"]),
        )


@dataclass(frozen=True, slots=True)
class ScoresBatchSummary:
    fixture_id: int
    update_stats: UpdateStats
    event_stats_sub_tree_root: Hash32

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ScoresBatchSummary:
        return cls(
            fixture_id=int(data["fixtureId"]),
            update_stats=UpdateStats.from_dict(data["updateStats"]),
            event_stats_sub_tree_root=Hash32.decode(data["eventStatsSubTreeRoot"]),
        )


@dataclass(frozen=True, slots=True)
class FixtureSummaryInput:
    fixture_id: int
    update_count: int
    min_timestamp: int
    max_timestamp: int
    events_sub_tree_root: bytes


@dataclass(frozen=True, slots=True)
class StatTermInput:
    stat_to_prove: ScoreStat
    event_stat_root: bytes
    stat_proof: list[ProofNode]


@dataclass(frozen=True, slots=True)
class ScoresStatValidation:
    ts: int
    stat_to_prove: ScoreStat
    event_stat_root: Hash32
    summary: ScoresBatchSummary
    stat_proof: list[ProofNode]
    sub_tree_proof: list[ProofNode]
    main_tree_proof: list[ProofNode]
    stat_to_prove2: ScoreStat | None = None
    stat_proof2: list[ProofNode] | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ScoresStatValidation:
        return cls(
            ts=int(data["ts"]),
            stat_to_prove=ScoreStat.from_dict(data["statToProve"]),
            event_stat_root=Hash32.decode(data["eventStatRoot"]),
            summary=ScoresBatchSummary.from_dict(data["summary"]),
            stat_proof=proof_nodes_from_json(data.get("statProof")),
            sub_tree_proof=proof_nodes_from_json(data.get("subTreeProof")),
            main_tree_proof=proof_nodes_from_json(data.get("mainTreeProof")),
            stat_to_prove2=(
                ScoreStat.from_dict(data["statToProve2"]) if data.get("statToProve2") else None
            ),
            stat_proof2=(
                proof_nodes_from_json(data["statProof2"])
                if data.get("statProof2") is not None
                else None
            ),
        )

    def fixture_summary_input(self) -> FixtureSummaryInput:
        return FixtureSummaryInput(
            fixture_id=self.summary.fixture_id,
            update_count=self.summary.update_stats.update_count,
            min_timestamp=self.summary.update_stats.min_timestamp,
            max_timestamp=self.summary.update_stats.max_timestamp,
            events_sub_tree_root=self.summary.event_stats_sub_tree_root.as_bytes(),
        )

    def primary_stat_term(self) -> StatTermInput:
        return StatTermInput(
            stat_to_prove=self.stat_to_prove,
            event_stat_root=self.event_stat_root.as_bytes(),
            stat_proof=self.stat_proof,
        )

    def secondary_stat_term(self) -> StatTermInput | None:
        if self.stat_to_prove2 is None and self.stat_proof2 is None:
            return None
        if self.stat_to_prove2 is None or self.stat_proof2 is None:
            raise ValidationError("legacy response contains only one of statToProve2/statProof2")
        return StatTermInput(
            stat_to_prove=self.stat_to_prove2,
            event_stat_root=self.event_stat_root.as_bytes(),
            stat_proof=self.stat_proof2,
        )

    def epoch_day(self) -> int:
        return timestamp_ms_to_epoch_day(self.summary.update_stats.min_timestamp)


def ensure_positive_seq(seq: int) -> None:
    if seq <= 0:
        raise InvalidInputError(
            "score stat validation seq must be greater than zero and must come from a real "
            "score record"
        )


def timestamp_ms_to_epoch_day(timestamp_ms: int) -> int:
    if timestamp_ms < 0:
        raise ValidationError("validation timestamp must not be negative")
    epoch_day = timestamp_ms // 86_400_000
    if epoch_day > 0xFFFF:
        raise ValidationError("epoch day does not fit into u16 PDA seed")
    return epoch_day
