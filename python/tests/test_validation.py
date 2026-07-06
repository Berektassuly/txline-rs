from __future__ import annotations

import base64
from dataclasses import replace

import pytest

from txline.errors import InvalidInputError, ValidationError
from txline.validation import (
    BinaryExpression,
    Comparison,
    Hash32,
    NDimensionalStrategy,
    ScoresBatchSummary,
    ScoresStatValidationV2,
    ScoresStatValidationV2Response,
    ScoreStat,
    TraderPredicate,
)
from txline.validation.legacy import UpdateStats, ensure_positive_seq


def test_proof_hash_decoding_accepts_base64_hex_and_byte_arrays() -> None:
    raw = bytes([7]) * 32
    assert Hash32.decode(base64.b64encode(raw).decode()).as_bytes() == raw
    assert Hash32.decode("0x" + "ab" * 32).as_bytes() == bytes([0xAB]) * 32
    assert Hash32.decode([3] * 32).as_bytes() == bytes([3]) * 32
    with pytest.raises(Exception, match="expected 32 bytes|hash string"):
        Hash32.decode([1] * 31)


def test_v2_stat_validation_shape_checks() -> None:
    validation = ScoresStatValidationV2.from_response([1001, 1002], _v2_response(2))
    assert validation.requested_stat_keys == [1001, 1002]
    assert validation.to_validation_input().stats[0].stat.key == 1001

    with pytest.raises(ValidationError, match="statsToProve length"):
        ScoresStatValidationV2.from_response([1001, 1002, 1003], _v2_response(2))

    response = _v2_response(2)
    response = replace(response, stats_to_prove=list(reversed(response.stats_to_prove)))
    with pytest.raises(ValidationError, match=r"statsToProve\[0\]\.key"):
        ScoresStatValidationV2.from_response([1001, 1002], response)

    bad = replace(_v2_response(2), stat_proofs=[[]])
    with pytest.raises(ValidationError, match="statProofs length"):
        ScoresStatValidationV2.from_response([1001, 1002], bad)


def test_seq_and_strategy_bounds_checks() -> None:
    with pytest.raises(InvalidInputError, match="seq must be greater than zero"):
        ensure_positive_seq(0)
    predicate = TraderPredicate(0, Comparison.EQUAL_TO)
    with pytest.raises(ValidationError, match="out of bounds"):
        NDimensionalStrategy.builder(2).binary(0, 2, BinaryExpression.SUBTRACT, predicate)
    strategy = (
        NDimensionalStrategy.builder(2)
        .single(0, TraderPredicate(1, Comparison.GREATER_THAN))
        .geometric_target(0, 0)
        .geometric_target(1, 1)
        .distance_predicate(TraderPredicate(2, Comparison.LESS_THAN))
        .build()
    )
    assert len(strategy.discrete_predicates) == 1
    assert len(strategy.geometric_targets) == 2


def _v2_response(count: int) -> ScoresStatValidationV2Response:
    hash32 = Hash32(bytes([9]) * 32)
    return ScoresStatValidationV2Response(
        ts=2,
        stats_to_prove=[ScoreStat(key=1001 + idx, value=idx, period=0) for idx in range(count)],
        event_stat_root=hash32,
        summary=ScoresBatchSummary(
            fixture_id=1,
            update_stats=UpdateStats(update_count=1, min_timestamp=1, max_timestamp=2),
            event_stats_sub_tree_root=hash32,
        ),
        stat_proofs=[[] for _ in range(count)],
        sub_tree_proof=[],
        main_tree_proof=[],
    )
