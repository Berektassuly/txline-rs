"""Strategy types for validate_stat_v2."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from txline.errors import ValidationError


class Comparison(str, Enum):
    GREATER_THAN = "greaterThan"
    LESS_THAN = "lessThan"
    EQUAL_TO = "equalTo"


class BinaryExpression(str, Enum):
    ADD = "add"
    SUBTRACT = "subtract"


@dataclass(frozen=True, slots=True)
class TraderPredicate:
    threshold: int
    comparison: Comparison


@dataclass(frozen=True, slots=True)
class SinglePredicate:
    index: int
    predicate: TraderPredicate


@dataclass(frozen=True, slots=True)
class BinaryPredicate:
    index_a: int
    index_b: int
    op: BinaryExpression
    predicate: TraderPredicate


StatPredicate = SinglePredicate | BinaryPredicate


@dataclass(frozen=True, slots=True)
class GeometricTarget:
    stat_index: int
    prediction: int


@dataclass(frozen=True, slots=True)
class NDimensionalStrategy:
    geometric_targets: list[GeometricTarget]
    distance_predicate: TraderPredicate | None
    discrete_predicates: list[StatPredicate]

    @classmethod
    def builder(cls, stat_count: int) -> StrategyBuilder:
        return StrategyBuilder(stat_count)

    def validate_indices(self, stat_count: int) -> None:
        for target in self.geometric_targets:
            _ensure_index(target.stat_index, stat_count)
        for predicate in self.discrete_predicates:
            if isinstance(predicate, SinglePredicate):
                _ensure_index(predicate.index, stat_count)
            else:
                _ensure_index(predicate.index_a, stat_count)
                _ensure_index(predicate.index_b, stat_count)
        if self.geometric_targets and self.distance_predicate is None:
            raise ValidationError("geometric targets require a distance predicate")


class StrategyBuilder:
    def __init__(self, stat_count: int) -> None:
        self._stat_count = stat_count
        self._geometric_targets: list[GeometricTarget] = []
        self._distance_predicate: TraderPredicate | None = None
        self._discrete_predicates: list[StatPredicate] = []

    def single(self, index: int, predicate: TraderPredicate) -> StrategyBuilder:
        _ensure_index(index, self._stat_count)
        self._discrete_predicates.append(SinglePredicate(index=index, predicate=predicate))
        return self

    def binary(
        self,
        index_a: int,
        index_b: int,
        op: BinaryExpression,
        predicate: TraderPredicate,
    ) -> StrategyBuilder:
        _ensure_index(index_a, self._stat_count)
        _ensure_index(index_b, self._stat_count)
        self._discrete_predicates.append(
            BinaryPredicate(index_a=index_a, index_b=index_b, op=op, predicate=predicate)
        )
        return self

    def geometric_target(self, stat_index: int, prediction: int) -> StrategyBuilder:
        _ensure_index(stat_index, self._stat_count)
        self._geometric_targets.append(
            GeometricTarget(stat_index=stat_index, prediction=prediction)
        )
        return self

    def distance_predicate(self, predicate: TraderPredicate) -> StrategyBuilder:
        self._distance_predicate = predicate
        return self

    def build(self) -> NDimensionalStrategy:
        strategy = NDimensionalStrategy(
            geometric_targets=list(self._geometric_targets),
            distance_predicate=self._distance_predicate,
            discrete_predicates=list(self._discrete_predicates),
        )
        strategy.validate_indices(self._stat_count)
        return strategy


def _ensure_index(index: int, stat_count: int) -> None:
    if index < 0 or index >= stat_count:
        raise ValidationError(
            f"strategy index {index} is out of bounds for {stat_count} requested stat keys"
        )
