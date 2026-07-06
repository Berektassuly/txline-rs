"""Hackathon-oriented Devnet trading lifecycle helpers."""

from __future__ import annotations

from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from enum import Enum

from txline.errors import InvalidInputError, ValidationError
from txline.scores import Scores
from txline.solana.instructions import (
    AuditTradeResultAccounts,
    AuditTradeResultParams,
    ClaimBatchLegacyAccounts,
    ClaimBatchLegacyParams,
    ClaimViaResolutionAccounts,
    ClaimViaResolutionParams,
    CloseIntentAccounts,
    CreateIntentAccounts,
    CreateIntentParams,
    CreateTradeAccounts,
    CreateTradeParams,
    ExecuteMatchAccounts,
    ExecuteMatchParams,
    MarketIntentParams,
    RefundBatchAccounts,
    SettleMatchedTradeAccounts,
    SettleMatchedTradeParams,
    SettleTradeAccounts,
    SettleTradeParams,
    audit_trade_result_instruction,
    claim_batch_legacy_instruction,
    claim_via_resolution_instruction,
    close_intent_instruction,
    create_intent_instruction,
    create_trade_instruction,
    execute_match_instruction,
    refund_batch_instruction,
    settle_matched_trade_instruction,
    settle_trade_instruction,
)
from txline.solana.pda import Instruction, Pubkey, ensure_pubkey
from txline.validation.legacy import FixtureSummaryInput, StatTermInput
from txline.validation.proof import ProofNode
from txline.validation.strategy import (
    BinaryExpression,
    Comparison,
    NDimensionalStrategy,
    TraderPredicate,
)
from txline.validation.v2 import ScoresStatValidationV2, StatValidationInput


class MarketSide(str, Enum):
    """A side in a score-based prediction market."""

    PARTICIPANT1 = "participant1"
    PARTICIPANT2 = "participant2"
    DRAW = "draw"


class ScoreMarketKind(str, Enum):
    """Supported score-market shapes backed by Devnet score-stat proofs."""

    FINAL_OUTCOME = "final_outcome"
    TOTAL_GOALS = "total_goals"
    SPREAD = "spread"


@dataclass(frozen=True, slots=True)
class FinalOutcomeConfig:
    """Stat keys and period used for soccer final-outcome settlement."""

    participant1_goals_stat_key: int = 1
    participant2_goals_stat_key: int = 2
    period: int = 100


@dataclass(frozen=True, slots=True)
class FinalOutcome:
    """Final score observation extracted from a TxLINE score record."""

    fixture_id: int
    seq: int
    participant1_score: int
    participant2_score: int
    side: MarketSide
    config: FinalOutcomeConfig


@dataclass(frozen=True, slots=True)
class ScoreMarketTerms:
    """Devnet market terms that map directly to `MarketIntentParams`."""

    fixture_id: int
    kind: ScoreMarketKind
    period: int
    stat_a_key: int
    stat_b_key: int | None
    predicate: TraderPredicate
    op: BinaryExpression | None
    negation: bool = False

    def __post_init__(self) -> None:
        if self.fixture_id <= 0:
            raise InvalidInputError("fixture_id must be positive")
        if self.period < 0 or self.period > 0xFFFF:
            raise InvalidInputError("period must fit into the Devnet IDL u16 field")
        if self.stat_a_key < 0:
            raise InvalidInputError("stat_a_key must be nonnegative")
        if self.stat_b_key is not None and self.stat_b_key < 0:
            raise InvalidInputError("stat_b_key must be nonnegative")
        if (self.stat_b_key is None) != (self.op is None):
            raise InvalidInputError("stat_b_key and op must either both be set or both be absent")

    def stat_keys(self) -> tuple[int, ...]:
        if self.stat_b_key is None:
            return (self.stat_a_key,)
        return (self.stat_a_key, self.stat_b_key)

    def to_market_intent_params(self) -> MarketIntentParams:
        return MarketIntentParams(
            fixture_id=self.fixture_id,
            period=self.period,
            stat_a_key=self.stat_a_key,
            stat_b_key=self.stat_b_key,
            predicate=self.predicate,
            op=self.op,
            negation=self.negation,
        )


@dataclass(frozen=True, slots=True)
class TermsHash:
    """Caller-provided 32-byte terms hash.

    The Devnet IDL requires a 32-byte hash, but the production preimage format is
    application-owned and not published in the TxLINE OpenAPI or Devnet IDL.
    """

    _bytes: bytes

    def __init__(self, value: bytes | bytearray | Iterable[int]) -> None:
        try:
            raw = bytes(value)
        except TypeError as exc:
            raise InvalidInputError(
                "terms_hash must be bytes-like or an iterable of bytes"
            ) from exc
        if len(raw) != 32:
            raise InvalidInputError(
                "terms_hash must be exactly 32 bytes; pass the caller-provided market hash"
            )
        object.__setattr__(self, "_bytes", raw)

    def as_bytes(self) -> bytes:
        return self._bytes

    def __bytes__(self) -> bytes:
        return self._bytes


TermsHashLike = TermsHash | bytes | bytearray | Iterable[int]


@dataclass(frozen=True, slots=True)
class LifecyclePlan:
    """Ordered instructions plus caller-owned lifecycle boundaries."""

    name: str
    instructions: tuple[Instruction, ...]
    next_steps: tuple[str, ...] = ()
    caller_boundaries: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class SettlementProofInputs:
    """Proof material arranged for Devnet trading settlement instructions."""

    ts: int
    fixture_summary: FixtureSummaryInput
    fixture_proof: list[ProofNode]
    main_tree_proof: list[ProofNode]
    stat_a: StatTermInput
    stat_b: StatTermInput | None


def default_soccer_final_outcome_config() -> FinalOutcomeConfig:
    return FinalOutcomeConfig()


def is_final_outcome_record(score: Scores) -> bool:
    return score.is_final_outcome_record()


def extract_final_outcome(score: Scores, config: FinalOutcomeConfig | None = None) -> FinalOutcome:
    cfg = config or default_soccer_final_outcome_config()
    if not is_final_outcome_record(score):
        raise InvalidInputError(
            "score record is not final-outcome settlement data; expected "
            "action=game_finalised, statusId=100, period=100"
        )
    if score.seq <= 0:
        raise InvalidInputError("final outcome seq must be positive")
    participant1_score = _score_stat_value(score, cfg.participant1_goals_stat_key)
    participant2_score = _score_stat_value(score, cfg.participant2_goals_stat_key)
    if participant1_score > participant2_score:
        side = MarketSide.PARTICIPANT1
    elif participant2_score > participant1_score:
        side = MarketSide.PARTICIPANT2
    else:
        side = MarketSide.DRAW
    return FinalOutcome(
        fixture_id=score.fixture_id,
        seq=score.seq,
        participant1_score=participant1_score,
        participant2_score=participant2_score,
        side=side,
        config=cfg,
    )


def final_outcome_stat_keys(config: FinalOutcomeConfig | None = None) -> list[int]:
    cfg = config or default_soccer_final_outcome_config()
    return [cfg.participant1_goals_stat_key, cfg.participant2_goals_stat_key]


def score_market_stat_keys(terms: ScoreMarketTerms) -> list[int]:
    return list(terms.stat_keys())


def final_outcome_market_terms(
    fixture_id: int,
    side: MarketSide,
    config: FinalOutcomeConfig | None = None,
) -> ScoreMarketTerms:
    cfg = config or default_soccer_final_outcome_config()
    greater_than_zero = TraderPredicate(0, Comparison.GREATER_THAN)
    if side == MarketSide.PARTICIPANT1:
        return ScoreMarketTerms(
            fixture_id=fixture_id,
            kind=ScoreMarketKind.FINAL_OUTCOME,
            period=cfg.period,
            stat_a_key=cfg.participant1_goals_stat_key,
            stat_b_key=cfg.participant2_goals_stat_key,
            predicate=greater_than_zero,
            op=BinaryExpression.SUBTRACT,
        )
    if side == MarketSide.PARTICIPANT2:
        return ScoreMarketTerms(
            fixture_id=fixture_id,
            kind=ScoreMarketKind.FINAL_OUTCOME,
            period=cfg.period,
            stat_a_key=cfg.participant2_goals_stat_key,
            stat_b_key=cfg.participant1_goals_stat_key,
            predicate=greater_than_zero,
            op=BinaryExpression.SUBTRACT,
        )
    return ScoreMarketTerms(
        fixture_id=fixture_id,
        kind=ScoreMarketKind.FINAL_OUTCOME,
        period=cfg.period,
        stat_a_key=cfg.participant1_goals_stat_key,
        stat_b_key=cfg.participant2_goals_stat_key,
        predicate=TraderPredicate(0, Comparison.EQUAL_TO),
        op=BinaryExpression.SUBTRACT,
    )


def total_goals_market_terms(
    fixture_id: int,
    predicate: TraderPredicate,
    config: FinalOutcomeConfig | None = None,
) -> ScoreMarketTerms:
    cfg = config or default_soccer_final_outcome_config()
    return ScoreMarketTerms(
        fixture_id=fixture_id,
        kind=ScoreMarketKind.TOTAL_GOALS,
        period=cfg.period,
        stat_a_key=cfg.participant1_goals_stat_key,
        stat_b_key=cfg.participant2_goals_stat_key,
        predicate=predicate,
        op=BinaryExpression.ADD,
    )


def spread_market_terms(
    fixture_id: int,
    side: MarketSide,
    predicate: TraderPredicate,
    config: FinalOutcomeConfig | None = None,
) -> ScoreMarketTerms:
    if side == MarketSide.DRAW:
        raise InvalidInputError("spread markets require participant1 or participant2")
    cfg = config or default_soccer_final_outcome_config()
    if side == MarketSide.PARTICIPANT1:
        stat_a_key = cfg.participant1_goals_stat_key
        stat_b_key = cfg.participant2_goals_stat_key
    else:
        stat_a_key = cfg.participant2_goals_stat_key
        stat_b_key = cfg.participant1_goals_stat_key
    return ScoreMarketTerms(
        fixture_id=fixture_id,
        kind=ScoreMarketKind.SPREAD,
        period=cfg.period,
        stat_a_key=stat_a_key,
        stat_b_key=stat_b_key,
        predicate=predicate,
        op=BinaryExpression.SUBTRACT,
    )


def final_outcome_strategy(outcome: FinalOutcome) -> NDimensionalStrategy:
    if outcome.side == MarketSide.PARTICIPANT1:
        return (
            NDimensionalStrategy.builder(2)
            .binary(
                0,
                1,
                BinaryExpression.SUBTRACT,
                TraderPredicate(0, Comparison.GREATER_THAN),
            )
            .build()
        )
    if outcome.side == MarketSide.PARTICIPANT2:
        return (
            NDimensionalStrategy.builder(2)
            .binary(
                1,
                0,
                BinaryExpression.SUBTRACT,
                TraderPredicate(0, Comparison.GREATER_THAN),
            )
            .build()
        )
    return (
        NDimensionalStrategy.builder(2)
        .binary(0, 1, BinaryExpression.SUBTRACT, TraderPredicate(0, Comparison.EQUAL_TO))
        .build()
    )


def market_terms_strategy(
    terms: ScoreMarketTerms, requested_stat_keys: Sequence[int]
) -> NDimensionalStrategy:
    if terms.negation:
        raise ValidationError(
            "N-dimensional validation strategies do not encode MarketIntentParams.negation; "
            "express the predicate directly before building a validation instruction"
        )
    index_a = _index_of_stat_key(requested_stat_keys, terms.stat_a_key)
    builder = NDimensionalStrategy.builder(len(requested_stat_keys))
    if terms.stat_b_key is None:
        return builder.single(index_a, terms.predicate).build()
    if terms.op is None:
        raise ValidationError("binary market terms require an operator")
    index_b = _index_of_stat_key(requested_stat_keys, terms.stat_b_key)
    return builder.binary(index_a, index_b, terms.op, terms.predicate).build()


def validation_input_for_market(
    validation: ScoresStatValidationV2, terms: ScoreMarketTerms
) -> StatValidationInput:
    missing = [key for key in terms.stat_keys() if key not in validation.requested_stat_keys]
    if missing:
        raise ValidationError(f"V2 validation payload is missing stat keys {missing}")
    return validation.to_validation_input()


def settlement_inputs_from_v2(
    payload: StatValidationInput, terms: ScoreMarketTerms
) -> SettlementProofInputs:
    return SettlementProofInputs(
        ts=payload.ts,
        fixture_summary=payload.fixture_summary,
        fixture_proof=payload.fixture_proof,
        main_tree_proof=payload.main_tree_proof,
        stat_a=_stat_term_from_payload(payload, terms.stat_a_key, terms.period),
        stat_b=(
            _stat_term_from_payload(payload, terms.stat_b_key, terms.period)
            if terms.stat_b_key is not None
            else None
        ),
    )


def ensure_terms_hash(value: TermsHashLike) -> bytes:
    if isinstance(value, TermsHash):
        return value.as_bytes()
    return TermsHash(value).as_bytes()


def create_intent_plan(
    program_id: str | Pubkey,
    accounts: CreateIntentAccounts,
    *,
    intent_id: int,
    terms_hash: TermsHashLike,
    deposit_amount: int,
    expiration_ts: int,
    claim_period: int,
    terms: ScoreMarketTerms,
) -> LifecyclePlan:
    params = CreateIntentParams(
        intent_id=intent_id,
        terms_hash=ensure_terms_hash(terms_hash),
        deposit_amount=deposit_amount,
        expiration_ts=expiration_ts,
        claim_period=claim_period,
        fixture_id=terms.fixture_id,
    )
    ix = create_intent_instruction(ensure_pubkey(program_id), accounts, params)
    return _single_instruction_plan(
        "create_intent",
        ix,
        (
            "The maker signs and submits the instruction with caller-supplied intent accounts.",
            "The coordinating application keeps the terms hash preimage and intent metadata.",
        ),
    )


def close_intent_plan(program_id: str | Pubkey, accounts: CloseIntentAccounts) -> LifecyclePlan:
    return _single_instruction_plan(
        "close_intent",
        close_intent_instruction(ensure_pubkey(program_id), accounts),
        ("The authority signs and submits the close instruction for the explicit intent account.",),
    )


def create_trade_plan(
    program_id: str | Pubkey,
    accounts: CreateTradeAccounts,
    *,
    trade_id: int,
    stake_a: int,
    stake_b: int,
    trade_terms_hash: TermsHashLike,
) -> LifecyclePlan:
    params = CreateTradeParams(
        trade_id=trade_id,
        stake_a=stake_a,
        stake_b=stake_b,
        trade_terms_hash=ensure_terms_hash(trade_terms_hash),
    )
    return _single_instruction_plan(
        "create_trade",
        create_trade_instruction(ensure_pubkey(program_id), accounts, params),
        (
            "Both traders and the authority must sign according to the Devnet "
            "instruction account metas.",
        ),
    )


def execute_match_plan(
    program_id: str | Pubkey,
    accounts: ExecuteMatchAccounts,
    params: ExecuteMatchParams,
) -> LifecyclePlan:
    return _single_instruction_plan(
        "execute_match",
        execute_match_instruction(ensure_pubkey(program_id), accounts, params),
        ("The solver submits the match using caller-supplied maker and taker intent accounts.",),
    )


def settle_trade_plan(
    program_id: str | Pubkey,
    accounts: SettleTradeAccounts,
    *,
    trade_id: int,
    validation_input: StatValidationInput,
    terms: ScoreMarketTerms,
) -> LifecyclePlan:
    settlement = settlement_inputs_from_v2(validation_input, terms)
    params = SettleTradeParams(
        trade_id=trade_id,
        ts=settlement.ts,
        fixture_summary=settlement.fixture_summary,
        fixture_proof=settlement.fixture_proof,
        main_tree_proof=settlement.main_tree_proof,
        predicate=terms.predicate,
        stat_a=settlement.stat_a,
        stat_b=settlement.stat_b,
        op=terms.op,
    )
    return _single_instruction_plan(
        "settle_trade",
        settle_trade_instruction(ensure_pubkey(program_id), accounts, params),
        (
            "The winner signs and submits the direct-trade settlement instruction.",
            "The proof payload must come from the TxLINE stat-validation endpoint "
            "for the observed score seq.",
        ),
    )


def settle_matched_trade_plan(
    program_id: str | Pubkey,
    accounts: SettleMatchedTradeAccounts,
    *,
    trade_id: int,
    validation_input: StatValidationInput,
    terms: ScoreMarketTerms,
) -> LifecyclePlan:
    settlement = settlement_inputs_from_v2(validation_input, terms)
    params = SettleMatchedTradeParams(
        trade_id=trade_id,
        ts=settlement.ts,
        fixture_summary=settlement.fixture_summary,
        fixture_proof=settlement.fixture_proof,
        main_tree_proof=settlement.main_tree_proof,
        stat_a=settlement.stat_a,
        stat_b=settlement.stat_b,
        terms=terms.to_market_intent_params(),
    )
    return _single_instruction_plan(
        "settle_matched_trade",
        settle_matched_trade_instruction(ensure_pubkey(program_id), accounts, params),
        (
            "The winner signs and submits the matched-trade settlement instruction.",
            "The application supplies matched trade, vault, token, and winner token accounts.",
        ),
    )


def claim_via_resolution_plan(
    program_id: str | Pubkey,
    accounts: ClaimViaResolutionAccounts,
    params: ClaimViaResolutionParams,
) -> LifecyclePlan:
    return _single_instruction_plan(
        "claim_via_resolution",
        claim_via_resolution_instruction(ensure_pubkey(program_id), accounts, params),
        ("The caller supplies the resolution-root account and Merkle proof.",),
    )


def claim_batch_legacy_plan(
    program_id: str | Pubkey,
    accounts: ClaimBatchLegacyAccounts,
    *,
    epoch_day: int,
    interval_index: int,
    terms_hash: TermsHashLike,
    winner_is_maker: bool,
    seq: int,
    merkle_proof: list[ProofNode],
) -> LifecyclePlan:
    params = ClaimBatchLegacyParams(
        epoch_day=epoch_day,
        interval_index=interval_index,
        terms_hash=ensure_terms_hash(terms_hash),
        winner_is_maker=winner_is_maker,
        seq=seq,
        merkle_proof=merkle_proof,
    )
    return _single_instruction_plan(
        "claim_batch_legacy",
        claim_batch_legacy_instruction(ensure_pubkey(program_id), accounts, params),
        ("The caller supplies the legacy resolution proof and token accounts.",),
    )


def refund_batch_plan(program_id: str | Pubkey, accounts: RefundBatchAccounts) -> LifecyclePlan:
    return _single_instruction_plan(
        "refund_batch",
        refund_batch_instruction(ensure_pubkey(program_id), accounts),
        ("The payer signs the refund instruction for the caller-supplied token accounts.",),
    )


def audit_trade_result_plan(
    program_id: str | Pubkey,
    accounts: AuditTradeResultAccounts,
    *,
    validation_input: StatValidationInput,
    terms: ScoreMarketTerms,
) -> LifecyclePlan:
    settlement = settlement_inputs_from_v2(validation_input, terms)
    params = AuditTradeResultParams(
        terms=terms.to_market_intent_params(),
        fixture_summary=settlement.fixture_summary,
        main_tree_proof=settlement.main_tree_proof,
        fixture_proof=settlement.fixture_proof,
        stat_a=settlement.stat_a,
        stat_b=settlement.stat_b,
        ts=settlement.ts,
    )
    return _single_instruction_plan(
        "audit_trade_result",
        audit_trade_result_instruction(ensure_pubkey(program_id), accounts, params),
        ("The payer signs a read/audit instruction against the caller-supplied scores root.",),
    )


def _single_instruction_plan(
    name: str,
    instruction: Instruction,
    next_steps: tuple[str, ...],
) -> LifecyclePlan:
    return LifecyclePlan(
        name=name,
        instructions=(instruction,),
        next_steps=next_steps,
        caller_boundaries=(
            "Trading PDAs, escrow accounts, vaults, signers, and terms-hash "
            "preimages are supplied by the application.",
            "This SDK builds Devnet instructions only; it does not sign, simulate, "
            "or submit transactions.",
        ),
    )


def _score_stat_value(score: Scores, stat_key: int) -> int:
    if score.stats is None:
        raise InvalidInputError("final outcome score record has no stats payload")
    text_key = str(stat_key)
    if text_key not in score.stats:
        raise InvalidInputError(f"final outcome score record is missing stat key {stat_key}")
    return score.stats[text_key]


def _index_of_stat_key(requested_stat_keys: Sequence[int], stat_key: int) -> int:
    for idx, requested in enumerate(requested_stat_keys):
        if requested == stat_key:
            return idx
    raise ValidationError(f"requested stat keys do not include market stat key {stat_key}")


def _stat_term_from_payload(
    payload: StatValidationInput, stat_key: int, expected_period: int
) -> StatTermInput:
    matches = [leaf for leaf in payload.stats if leaf.stat.key == stat_key]
    if not matches:
        raise ValidationError(f"validation payload does not contain stat key {stat_key}")
    if len(matches) > 1:
        raise ValidationError(f"validation payload contains duplicate stat key {stat_key}")
    leaf = matches[0]
    if leaf.stat.period != expected_period:
        raise ValidationError(
            f"stat key {stat_key} period {leaf.stat.period} does not match market period "
            f"{expected_period}"
        )
    return StatTermInput(
        stat_to_prove=leaf.stat,
        event_stat_root=payload.event_stat_root,
        stat_proof=leaf.stat_proof,
    )


__all__ = [
    "FinalOutcome",
    "FinalOutcomeConfig",
    "LifecyclePlan",
    "MarketSide",
    "ScoreMarketKind",
    "ScoreMarketTerms",
    "SettlementProofInputs",
    "TermsHash",
    "audit_trade_result_plan",
    "claim_batch_legacy_plan",
    "claim_via_resolution_plan",
    "close_intent_plan",
    "create_intent_plan",
    "create_trade_plan",
    "default_soccer_final_outcome_config",
    "ensure_terms_hash",
    "execute_match_plan",
    "extract_final_outcome",
    "final_outcome_market_terms",
    "final_outcome_stat_keys",
    "final_outcome_strategy",
    "is_final_outcome_record",
    "market_terms_strategy",
    "refund_batch_plan",
    "score_market_stat_keys",
    "settle_matched_trade_plan",
    "settle_trade_plan",
    "settlement_inputs_from_v2",
    "spread_market_terms",
    "total_goals_market_terms",
    "validation_input_for_market",
]
