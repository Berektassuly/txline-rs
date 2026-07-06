"""Validation DTOs, proof decoding, and V2 strategy helpers."""

from txline.validation.legacy import (
    FixtureSummaryInput,
    ScoresBatchSummary,
    ScoresStatValidation,
    ScoreStat,
    StatTermInput,
    ensure_positive_seq,
    timestamp_ms_to_epoch_day,
)
from txline.validation.proof import Hash32, ProofNode
from txline.validation.strategy import (
    BinaryExpression,
    Comparison,
    GeometricTarget,
    NDimensionalStrategy,
    StatPredicate,
    StrategyBuilder,
    TraderPredicate,
)
from txline.validation.v2 import (
    ScoresStatValidationV2,
    ScoresStatValidationV2Response,
    StatLeafInput,
    StatValidationInput,
)

__all__ = [
    "BinaryExpression",
    "Comparison",
    "FixtureSummaryInput",
    "GeometricTarget",
    "Hash32",
    "NDimensionalStrategy",
    "ProofNode",
    "ScoreStat",
    "ScoresBatchSummary",
    "ScoresStatValidation",
    "ScoresStatValidationV2",
    "ScoresStatValidationV2Response",
    "StatLeafInput",
    "StatPredicate",
    "StatTermInput",
    "StatValidationInput",
    "StrategyBuilder",
    "TraderPredicate",
    "ensure_positive_seq",
    "timestamp_ms_to_epoch_day",
]
