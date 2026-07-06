"""V2 score stat validation DTOs."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from txline.errors import InvalidInputError, ValidationError
from txline.validation.legacy import FixtureSummaryInput, ScoresBatchSummary, ScoreStat
from txline.validation.proof import Hash32, ProofNode, proof_nodes_from_json


@dataclass(frozen=True, slots=True)
class ScoresStatValidationV2Response:
    ts: int
    stats_to_prove: list[ScoreStat]
    event_stat_root: Hash32
    summary: ScoresBatchSummary
    stat_proofs: list[list[ProofNode]]
    sub_tree_proof: list[ProofNode]
    main_tree_proof: list[ProofNode]

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ScoresStatValidationV2Response:
        return cls(
            ts=int(data["ts"]),
            stats_to_prove=[ScoreStat.from_dict(item) for item in data.get("statsToProve", [])],
            event_stat_root=Hash32.decode(data["eventStatRoot"]),
            summary=ScoresBatchSummary.from_dict(data["summary"]),
            stat_proofs=[proof_nodes_from_json(items) for items in data.get("statProofs", [])],
            sub_tree_proof=proof_nodes_from_json(data.get("subTreeProof")),
            main_tree_proof=proof_nodes_from_json(data.get("mainTreeProof")),
        )


@dataclass(frozen=True, slots=True)
class StatLeafInput:
    stat: ScoreStat
    stat_proof: list[ProofNode]


@dataclass(frozen=True, slots=True)
class StatValidationInput:
    ts: int
    fixture_summary: FixtureSummaryInput
    fixture_proof: list[ProofNode]
    main_tree_proof: list[ProofNode]
    event_stat_root: bytes
    stats: list[StatLeafInput]


@dataclass(frozen=True, slots=True)
class ScoresStatValidationV2:
    requested_stat_keys: list[int]
    response: ScoresStatValidationV2Response

    @classmethod
    def from_response(
        cls,
        requested_stat_keys: list[int],
        response: ScoresStatValidationV2Response | dict[str, Any],
    ) -> ScoresStatValidationV2:
        parsed = (
            response
            if isinstance(response, ScoresStatValidationV2Response)
            else ScoresStatValidationV2Response.from_dict(response)
        )
        if not requested_stat_keys:
            raise InvalidInputError("V2 stat validation requires at least one stat key")
        if len(parsed.stats_to_prove) != len(requested_stat_keys):
            raise ValidationError(
                f"statsToProve length {len(parsed.stats_to_prove)} does not match requested "
                f"statKeys length {len(requested_stat_keys)}"
            )
        for idx, (stat, requested_key) in enumerate(
            zip(parsed.stats_to_prove, requested_stat_keys, strict=False)
        ):
            if stat.key != requested_key:
                raise ValidationError(
                    f"statsToProve[{idx}].key {stat.key} does not match "
                    f"requested statKeys[{idx}] {requested_key}"
                )
        if len(parsed.stat_proofs) != len(parsed.stats_to_prove):
            raise ValidationError(
                f"statProofs length {len(parsed.stat_proofs)} does not match "
                f"statsToProve length {len(parsed.stats_to_prove)}"
            )
        return cls(requested_stat_keys=list(requested_stat_keys), response=parsed)

    def stats_to_prove(self) -> list[ScoreStat]:
        return self.response.stats_to_prove

    def stat_proofs(self) -> list[list[ProofNode]]:
        return self.response.stat_proofs

    def target_ts(self) -> int:
        return self.response.summary.update_stats.min_timestamp

    def to_validation_input(self) -> StatValidationInput:
        return StatValidationInput(
            ts=self.target_ts(),
            fixture_summary=FixtureSummaryInput(
                fixture_id=self.response.summary.fixture_id,
                update_count=self.response.summary.update_stats.update_count,
                min_timestamp=self.response.summary.update_stats.min_timestamp,
                max_timestamp=self.response.summary.update_stats.max_timestamp,
                events_sub_tree_root=self.response.summary.event_stats_sub_tree_root.as_bytes(),
            ),
            fixture_proof=self.response.sub_tree_proof,
            main_tree_proof=self.response.main_tree_proof,
            event_stat_root=self.response.event_stat_root.as_bytes(),
            stats=[
                StatLeafInput(stat=stat, stat_proof=proof)
                for stat, proof in zip(
                    self.response.stats_to_prove, self.response.stat_proofs, strict=False
                )
            ],
        )

    def leading_subset(self, length: int) -> StatValidationInput:
        if length == 0 or length > len(self.response.stats_to_prove):
            raise ValidationError("V2 payload subset length must be within the proved stat count")
        input_data = self.to_validation_input()
        return StatValidationInput(
            ts=input_data.ts,
            fixture_summary=input_data.fixture_summary,
            fixture_proof=input_data.fixture_proof,
            main_tree_proof=input_data.main_tree_proof,
            event_stat_root=input_data.event_stat_root,
            stats=input_data.stats[:length],
        )
