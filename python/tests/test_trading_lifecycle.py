from __future__ import annotations

from dataclasses import replace

import pytest

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
from txline.solana.pda import Pubkey
from txline.trading_lifecycle import (
    MarketSide,
    audit_trade_result_plan,
    claim_batch_legacy_plan,
    claim_via_resolution_plan,
    close_intent_plan,
    create_intent_plan,
    create_trade_plan,
    ensure_terms_hash,
    execute_match_plan,
    extract_final_outcome,
    final_outcome_market_terms,
    final_outcome_stat_keys,
    final_outcome_strategy,
    is_final_outcome_record,
    refund_batch_plan,
    settle_matched_trade_plan,
    settle_trade_plan,
    settlement_inputs_from_v2,
    validation_input_for_market,
)
from txline.validation.legacy import FixtureSummaryInput, ScoresBatchSummary, ScoreStat, UpdateStats
from txline.validation.proof import Hash32, ProofNode
from txline.validation.strategy import (
    BinaryExpression,
    BinaryPredicate,
    Comparison,
    TraderPredicate,
)
from txline.validation.v2 import (
    ScoresStatValidationV2,
    ScoresStatValidationV2Response,
    StatLeafInput,
    StatValidationInput,
)


def test_final_outcome_detection_requires_action_status_and_period() -> None:
    final = score_record(3, 1)
    assert is_final_outcome_record(final)

    assert not is_final_outcome_record(replace(final, action="score"))
    assert not is_final_outcome_record(replace(final, status_id=99))
    assert not is_final_outcome_record(replace(final, period=99))


def test_extract_final_outcome_handles_all_result_sides() -> None:
    participant1 = extract_final_outcome(score_record(3, 1))
    participant2 = extract_final_outcome(score_record(0, 2))
    draw = extract_final_outcome(score_record(1, 1))

    assert participant1.side == MarketSide.PARTICIPANT1
    assert participant1.participant1_score == 3
    assert participant1.participant2_score == 1
    assert participant2.side == MarketSide.PARTICIPANT2
    assert draw.side == MarketSide.DRAW
    assert draw.seq == 941


def test_extract_final_outcome_reports_non_final_and_missing_stats() -> None:
    with pytest.raises(InvalidInputError, match="not final-outcome"):
        extract_final_outcome(replace(score_record(1, 0), action="score"))

    with pytest.raises(InvalidInputError, match="no stats"):
        extract_final_outcome(replace(score_record(1, 0), stats=None))

    with pytest.raises(InvalidInputError, match="missing stat key 2"):
        extract_final_outcome(replace(score_record(1, 0), stats={"1": 1}))


def test_terms_hash_must_be_explicit_32_bytes() -> None:
    raw = hash_bytes(3)

    assert ensure_terms_hash(raw) == raw
    assert ensure_terms_hash(list(raw)) == raw

    with pytest.raises(InvalidInputError, match="exactly 32 bytes"):
        ensure_terms_hash(raw[:-1])


def test_final_outcome_terms_and_strategy_are_deterministic() -> None:
    terms = final_outcome_market_terms(17952170, MarketSide.PARTICIPANT2)
    params = terms.to_market_intent_params()

    assert terms.stat_keys() == (2, 1)
    assert params.fixture_id == 17952170
    assert params.period == 100
    assert params.stat_a_key == 2
    assert params.stat_b_key == 1
    assert params.op == BinaryExpression.SUBTRACT
    assert params.predicate == TraderPredicate(0, Comparison.GREATER_THAN)
    assert not params.negation

    p1_strategy = final_outcome_strategy(extract_final_outcome(score_record(2, 0)))
    p2_strategy = final_outcome_strategy(extract_final_outcome(score_record(0, 2)))
    draw_strategy = final_outcome_strategy(extract_final_outcome(score_record(2, 2)))

    assert_binary_strategy(p1_strategy, 0, 1, Comparison.GREATER_THAN)
    assert_binary_strategy(p2_strategy, 1, 0, Comparison.GREATER_THAN)
    assert_binary_strategy(draw_strategy, 0, 1, Comparison.EQUAL_TO)


def test_validation_payload_preserves_order_while_settlement_uses_market_key_order() -> None:
    terms = final_outcome_market_terms(17952170, MarketSide.PARTICIPANT2)
    validation = ScoresStatValidationV2.from_response(
        final_outcome_stat_keys(), v2_response([1, 2])
    )

    payload = validation_input_for_market(validation, terms)
    settlement = settlement_inputs_from_v2(payload, terms)

    assert [leaf.stat.key for leaf in payload.stats] == [1, 2]
    assert settlement.stat_a.stat_to_prove.key == 2
    assert settlement.stat_b is not None
    assert settlement.stat_b.stat_to_prove.key == 1

    missing_terms = replace(terms, stat_a_key=99)
    with pytest.raises(ValidationError, match="missing stat keys"):
        validation_input_for_market(validation, missing_terms)


def test_lifecycle_plans_match_low_level_instruction_builders() -> None:
    program_id = key(200)
    terms = final_outcome_market_terms(17952170, MarketSide.PARTICIPANT1)
    terms_hash = hash_bytes(100)
    validation_input = settlement_payload()

    create_intent_accounts = CreateIntentAccounts(
        key(1), key(2), key(3), key(4), key(5), key(6), key(7), key(8)
    )
    create_intent = create_intent_plan(
        program_id,
        create_intent_accounts,
        intent_id=9001,
        terms_hash=terms_hash,
        deposit_amount=123_456,
        expiration_ts=1_781_129_999_999,
        claim_period=42,
        terms=terms,
    )
    assert create_intent.instructions == (
        create_intent_instruction(
            program_id,
            create_intent_accounts,
            CreateIntentParams(9001, terms_hash, 123_456, 1_781_129_999_999, 42, 17952170),
        ),
    )

    close_accounts = CloseIntentAccounts(
        key(11), key(12), key(13), key(14), key(15), key(16), key(17), key(18)
    )
    assert close_intent_plan(program_id, close_accounts).instructions == (
        close_intent_instruction(program_id, close_accounts),
    )

    trade_accounts = CreateTradeAccounts(
        key(21),
        key(22),
        key(23),
        key(24),
        key(25),
        key(26),
        key(27),
        key(28),
        key(29),
        key(30),
        key(31),
    )
    create_trade = create_trade_plan(
        program_id,
        trade_accounts,
        trade_id=9002,
        stake_a=111,
        stake_b=222,
        trade_terms_hash=terms_hash,
    )
    assert create_trade.instructions == (
        create_trade_instruction(
            program_id, trade_accounts, CreateTradeParams(9002, 111, 222, terms_hash)
        ),
    )

    match_accounts = ExecuteMatchAccounts(
        key(41), key(42), key(43), key(44), key(45), key(46), key(47), key(48), key(49), key(50)
    )
    match_params = ExecuteMatchParams(9003, 333, 444)
    assert execute_match_plan(program_id, match_accounts, match_params).instructions == (
        execute_match_instruction(program_id, match_accounts, match_params),
    )

    settle_accounts = SettleTradeAccounts(
        key(61), key(62), key(63), key(64), key(65), key(66), key(67), key(68), key(69)
    )
    settlement = settlement_inputs_from_v2(validation_input, terms)
    settle = settle_trade_plan(
        program_id,
        settle_accounts,
        trade_id=9004,
        validation_input=validation_input,
        terms=terms,
    )
    assert settle.instructions == (
        settle_trade_instruction(
            program_id,
            settle_accounts,
            SettleTradeParams(
                9004,
                settlement.ts,
                settlement.fixture_summary,
                settlement.fixture_proof,
                settlement.main_tree_proof,
                terms.predicate,
                settlement.stat_a,
                settlement.stat_b,
                terms.op,
            ),
        ),
    )

    matched_accounts = SettleMatchedTradeAccounts(
        key(71), key(72), key(73), key(74), key(75), key(76), key(77), key(78), key(79)
    )
    matched = settle_matched_trade_plan(
        program_id,
        matched_accounts,
        trade_id=9005,
        validation_input=validation_input,
        terms=terms,
    )
    assert matched.instructions == (
        settle_matched_trade_instruction(
            program_id,
            matched_accounts,
            SettleMatchedTradeParams(
                9005,
                settlement.ts,
                settlement.fixture_summary,
                settlement.fixture_proof,
                settlement.main_tree_proof,
                settlement.stat_a,
                settlement.stat_b,
                terms.to_market_intent_params(),
            ),
        ),
    )

    claim_accounts = ClaimViaResolutionAccounts(
        key(81), key(82), key(83), key(84), key(85), key(86)
    )
    claim_params = ClaimViaResolutionParams(20_615, 17, [proof(70, False)])
    assert claim_via_resolution_plan(program_id, claim_accounts, claim_params).instructions == (
        claim_via_resolution_instruction(program_id, claim_accounts, claim_params),
    )

    legacy_accounts = ClaimBatchLegacyAccounts(key(91), key(92), key(93), key(94), key(95))
    legacy = claim_batch_legacy_plan(
        program_id,
        legacy_accounts,
        epoch_day=20_616,
        interval_index=18,
        terms_hash=terms_hash,
        winner_is_maker=True,
        seq=941,
        merkle_proof=[proof(72, False)],
    )
    assert legacy.instructions == (
        claim_batch_legacy_instruction(
            program_id,
            legacy_accounts,
            ClaimBatchLegacyParams(20_616, 18, terms_hash, True, 941, [proof(72, False)]),
        ),
    )

    refund_accounts = RefundBatchAccounts(key(101), key(102), key(103), key(104))
    assert refund_batch_plan(program_id, refund_accounts).instructions == (
        refund_batch_instruction(program_id, refund_accounts),
    )

    audit_accounts = AuditTradeResultAccounts(key(111), key(112))
    audit = audit_trade_result_plan(
        program_id,
        audit_accounts,
        validation_input=validation_input,
        terms=terms,
    )
    assert audit.instructions == (
        audit_trade_result_instruction(
            program_id,
            audit_accounts,
            AuditTradeResultParams(
                terms.to_market_intent_params(),
                settlement.fixture_summary,
                settlement.main_tree_proof,
                settlement.fixture_proof,
                settlement.stat_a,
                settlement.stat_b,
                settlement.ts,
            ),
        ),
    )


def score_record(participant1_goals: int, participant2_goals: int) -> Scores:
    return Scores(
        fixture_id=17952170,
        game_state="complete",
        start_time=1,
        is_team=True,
        fixture_group_id=1,
        competition_id=2,
        country_id=3,
        sport_id=4,
        participant1_is_home=True,
        participant2_id=20,
        participant1_id=10,
        action="game_finalised",
        id=99,
        ts=2,
        connection_id=77,
        seq=941,
        status_id=100,
        period=100,
        stats={"1": participant1_goals, "2": participant2_goals},
    )


def assert_binary_strategy(
    strategy: object, index_a: int, index_b: int, comparison: Comparison
) -> None:
    discrete = strategy.discrete_predicates  # type: ignore[attr-defined]
    assert len(discrete) == 1
    predicate = discrete[0]
    assert isinstance(predicate, BinaryPredicate)
    assert predicate.index_a == index_a
    assert predicate.index_b == index_b
    assert predicate.op == BinaryExpression.SUBTRACT
    assert predicate.predicate == TraderPredicate(0, comparison)


def v2_response(stat_keys: list[int]) -> ScoresStatValidationV2Response:
    hash32 = hash_obj(9)
    return ScoresStatValidationV2Response(
        ts=2,
        stats_to_prove=[
            ScoreStat(key=stat_key, value=idx + 1, period=100)
            for idx, stat_key in enumerate(stat_keys)
        ],
        event_stat_root=hash32,
        summary=ScoresBatchSummary(
            fixture_id=17952170,
            update_stats=UpdateStats(update_count=1, min_timestamp=1, max_timestamp=2),
            event_stats_sub_tree_root=hash32,
        ),
        stat_proofs=[[] for _ in stat_keys],
        sub_tree_proof=[],
        main_tree_proof=[],
    )


def settlement_payload() -> StatValidationInput:
    return StatValidationInput(
        ts=1_781_123_456_789,
        fixture_summary=FixtureSummaryInput(
            17952170,
            1,
            1_781_123_456_789,
            1_781_123_456_799,
            hash_bytes(10),
        ),
        fixture_proof=[proof(50, False)],
        main_tree_proof=[proof(60, True)],
        event_stat_root=hash_bytes(20),
        stats=[
            StatLeafInput(ScoreStat(1, 3, 100), [proof(30, True)]),
            StatLeafInput(ScoreStat(2, 1, 100), [proof(40, False)]),
        ],
    )


def proof(base: int, is_right_sibling: bool) -> ProofNode:
    return ProofNode(hash_obj(base), is_right_sibling)


def hash_obj(base: int) -> Hash32:
    return Hash32(hash_bytes(base))


def hash_bytes(base: int) -> bytes:
    return bytes((base + idx) % 256 for idx in range(32))


def key(base: int) -> Pubkey:
    return Pubkey(bytes([base]) * 32)
