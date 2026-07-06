from __future__ import annotations

import json
from dataclasses import replace
from pathlib import Path

import pytest

from txline.fixtures import (
    BatchMetadata,
    Fixture,
    FixtureBatchSummary,
    FixtureBatchValidation,
    FixtureValidation,
)
from txline.odds import OddsBatchSummary, OddsPayload, OddsValidation
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
    create_token_2022_associated_token_account_instruction,
    create_trade_instruction,
    execute_match_instruction,
    refund_batch_instruction,
    settle_matched_trade_instruction,
    settle_trade_instruction,
    validate_fixture_batch_instruction,
    validate_fixture_instruction,
    validate_odds_instruction,
    validate_stat_instruction,
    validate_stat_v2_instruction,
)
from txline.solana.pda import DevnetPdas, Pubkey
from txline.validation.legacy import (
    FixtureSummaryInput,
    ScoresBatchSummary,
    ScoresStatValidation,
    ScoreStat,
    StatTermInput,
    UpdateStats,
)
from txline.validation.proof import Hash32, ProofNode
from txline.validation.strategy import (
    BinaryExpression,
    BinaryPredicate,
    Comparison,
    GeometricTarget,
    NDimensionalStrategy,
    SinglePredicate,
    TraderPredicate,
)
from txline.validation.v2 import StatLeafInput, StatValidationInput


def test_known_devnet_pdas_match_rust() -> None:
    pdas = DevnetPdas.new()
    assert str(pdas.pricing_matrix().address) == "B4hHn1FpD1YPPrcM4yUrQhBPF18zFWgijHLTsumGzeKi"
    assert str(pdas.token_treasury_v2().address) == "Eqqd7rZQGzn2HA9L11NwBMhknxArM3L4KETyUuujK3LB"
    assert (
        str(pdas.token_treasury_vault_ata().address)
        == "dc6rQSPk8GJAeyyAtC1F62JoigmgEuLnW4k9zmgAeuM"
    )
    assert (
        str(pdas.daily_scores_roots(20_624).address)
        == "BM2n3RE2ADwZDaGehqd4mtkCQsFgW4aQrtdnDQ3VR4Kn"
    )
    assert (
        str(pdas.daily_batch_roots(20_624).address)
        == "2Y3dpLFRjA9M6J4MiAgrZ7AkPfzfke8Fzm2GLU92vTja"
    )
    assert (
        str(pdas.ten_daily_fixtures_roots(20_629).address)
        == "2ATJ2TkoB1c2PfTUqfDun1SwvnRNTTyGBt7D9h9Vkccd"
    )


def test_token_2022_ata_create_instruction_layout() -> None:
    ix = create_token_2022_associated_token_account_instruction(key(1), key(2), key(3), key(4))
    assert ix.data == b""
    assert ix.accounts[0].pubkey == key(1)
    assert ix.accounts[0].is_signer
    assert ix.accounts[1].is_writable
    assert not ix.accounts[5].is_writable


def test_validation_instruction_bytes_match_rust_golden() -> None:
    program_id = key(200)
    root = key(201)
    assert validate_stat_instruction(
        program_id,
        root,
        score_validation(),
        TraderPredicate(1, Comparison.LESS_THAN),
        BinaryExpression.ADD,
    ).data == golden("validation_golden.devnet.json", "validate_stat")
    assert validate_stat_v2_instruction(
        program_id, root, stat_v2_payload(), v2_strategy()
    ).data == golden("validation_golden.devnet.json", "validate_stat_v2")
    assert validate_fixture_instruction(program_id, root, fixture_validation()).data == golden(
        "validation_golden.devnet.json", "validate_fixture"
    )
    assert validate_fixture_batch_instruction(
        program_id, root, 3, fixture_batch_validation()
    ).data == golden("validation_golden.devnet.json", "validate_fixture_batch")
    assert validate_odds_instruction(program_id, root, odds_validation()).data == golden(
        "validation_golden.devnet.json", "validate_odds"
    )


def test_trading_instruction_bytes_match_rust_golden() -> None:
    program_id = key(200)
    cases = {
        "create_intent": create_intent_instruction(
            program_id, create_intent_accounts(), create_intent_params()
        ),
        "create_trade": create_trade_instruction(
            program_id, create_trade_accounts(), create_trade_params()
        ),
        "execute_match": execute_match_instruction(
            program_id, execute_match_accounts(), execute_match_params()
        ),
        "close_intent": close_intent_instruction(program_id, close_intent_accounts()),
        "settle_trade": settle_trade_instruction(
            program_id, settle_trade_accounts(), settle_trade_params()
        ),
        "settle_matched_trade": settle_matched_trade_instruction(
            program_id, settle_matched_trade_accounts(), settle_matched_trade_params()
        ),
        "claim_via_resolution": claim_via_resolution_instruction(
            program_id, claim_via_resolution_accounts(), claim_via_resolution_params()
        ),
        "claim_batch_legacy": claim_batch_legacy_instruction(
            program_id, claim_batch_legacy_accounts(), claim_batch_legacy_params()
        ),
        "refund_batch": refund_batch_instruction(program_id, refund_batch_accounts()),
        "audit_trade_result": audit_trade_result_instruction(
            program_id, audit_trade_result_accounts(), audit_trade_result_params()
        ),
    }
    for name, instruction in cases.items():
        assert instruction.data == golden("trading_golden.devnet.json", name)


def test_fixture_and_odds_reject_negative_unsigned_update_count() -> None:
    original = fixture_validation()
    bad_summary = replace(
        original.summary,
        update_stats=replace(original.summary.update_stats, update_count=-1),
    )
    bad_fixture = replace(original, summary=bad_summary)
    with pytest.raises(Exception, match="nonnegative"):
        validate_fixture_instruction(key(1), key(2), bad_fixture)


def score_validation() -> ScoresStatValidation:
    event_stat_root = hash32(20)
    return ScoresStatValidation(
        ts=1_781_123_456_789,
        stat_to_prove=ScoreStat(1001, 2, 0),
        event_stat_root=event_stat_root,
        summary=ScoresBatchSummary(
            fixture_id=I32_MAX + 6,
            update_stats=UpdateStats(-3, 1_781_123_456_789, 1_781_123_456_799),
            event_stats_sub_tree_root=hash32(10),
        ),
        stat_proof=[proof(30, True)],
        sub_tree_proof=[proof(50, False)],
        main_tree_proof=[proof(60, True)],
        stat_to_prove2=ScoreStat(1002, -1, 1),
        stat_proof2=[proof(40, False)],
    )


def stat_v2_payload() -> StatValidationInput:
    return StatValidationInput(
        ts=1_781_123_456_789,
        fixture_summary=FixtureSummaryInput(
            I32_MAX + 6, -3, 1_781_123_456_789, 1_781_123_456_799, hash_bytes(10)
        ),
        fixture_proof=[proof(51, False)],
        main_tree_proof=[proof(61, True)],
        event_stat_root=hash_bytes(22),
        stats=[
            StatLeafInput(ScoreStat(1001, 2, 0), [proof(31, True)]),
            StatLeafInput(ScoreStat(1002, -1, 1), [proof(41, False)]),
        ],
    )


def v2_strategy() -> NDimensionalStrategy:
    return NDimensionalStrategy(
        geometric_targets=[GeometricTarget(0, 0), GeometricTarget(1, 1)],
        distance_predicate=TraderPredicate(2, Comparison.LESS_THAN),
        discrete_predicates=[
            SinglePredicate(0, TraderPredicate(1, Comparison.EQUAL_TO)),
            BinaryPredicate(
                0, 1, BinaryExpression.SUBTRACT, TraderPredicate(0, Comparison.GREATER_THAN)
            ),
        ],
    )


def fixture_validation() -> FixtureValidation:
    return FixtureValidation(
        snapshot=Fixture(
            ts=1_781_123_000_000,
            start_time=1_781_126_600_000,
            competition="Devnet Cup",
            competition_id=7,
            fixture_group_id=-8,
            participant1_id=101,
            participant1="Alpha",
            participant2_id=202,
            participant2="Beta",
            fixture_id=I32_MAX + 7,
            participant1_is_home=True,
        ),
        summary=FixtureBatchSummary(
            fixture_id=I32_MAX + 7,
            competition_id=7,
            competition="Devnet Cup",
            update_stats=UpdateStats(4, 1_781_123_000_000, 1_781_123_000_001),
            update_sub_tree_root=hash32(70),
        ),
        sub_tree_proof=[proof(71, False)],
        main_tree_proof=[proof(72, True)],
    )


def fixture_batch_validation() -> FixtureBatchValidation:
    return FixtureBatchValidation(
        metadata=BatchMetadata(5, 2, 1_781_123_000_000, 1_781_123_900_000),
        proof=[proof(80, False), proof(81, True)],
    )


def odds_validation() -> OddsValidation:
    return OddsValidation(
        odds=OddsPayload(
            fixture_id=I32_MAX + 8,
            message_id="msg-1",
            ts=1_781_123_456_789,
            bookmaker="Book",
            bookmaker_id=9,
            super_odds_type="Winner",
            game_state="PreMatch",
            in_running=False,
            market_parameters=None,
            market_period="FT",
            price_names=["Home", "Away"],
            prices=[120, -125],
            pct=[],
        ),
        summary=OddsBatchSummary(
            fixture_id=I32_MAX + 8,
            update_stats=UpdateStats(5, 1_781_123_450_000, 1_781_123_459_999),
            odds_sub_tree_root=hash32(90),
        ),
        sub_tree_proof=[proof(91, False)],
        main_tree_proof=[proof(92, True)],
    )


def create_intent_accounts() -> CreateIntentAccounts:
    return CreateIntentAccounts(key(1), key(2), key(3), key(4), key(5), key(6), key(7), key(8))


def create_intent_params() -> CreateIntentParams:
    return CreateIntentParams(
        9001, hash_bytes(100), 123_456_789, 1_781_129_999_999, 42, I32_MAX + 6
    )


def create_trade_accounts() -> CreateTradeAccounts:
    return CreateTradeAccounts(
        key(11),
        key(12),
        key(13),
        key(14),
        key(15),
        key(16),
        key(17),
        key(18),
        key(19),
        key(20),
        key(21),
    )


def create_trade_params() -> CreateTradeParams:
    return CreateTradeParams(9002, 111_111, 222_222, hash_bytes(110))


def execute_match_accounts() -> ExecuteMatchAccounts:
    return ExecuteMatchAccounts(
        key(31), key(32), key(33), key(34), key(35), key(36), key(37), key(38), key(39), key(40)
    )


def execute_match_params() -> ExecuteMatchParams:
    return ExecuteMatchParams(9003, 333_333, 444_444)


def close_intent_accounts() -> CloseIntentAccounts:
    return CloseIntentAccounts(
        key(51), key(52), key(53), key(54), key(55), key(56), key(57), key(58)
    )


def settle_trade_accounts() -> SettleTradeAccounts:
    return SettleTradeAccounts(
        key(71), key(72), key(73), key(74), key(75), key(76), key(77), key(78), key(79)
    )


def settle_trade_params() -> SettleTradeParams:
    return SettleTradeParams(
        9004,
        1_781_123_456_789,
        fixture_summary(),
        [proof(50, False)],
        [proof(60, True)],
        TraderPredicate(1, Comparison.LESS_THAN),
        stat_a(),
        stat_b(),
        BinaryExpression.ADD,
    )


def settle_matched_trade_accounts() -> SettleMatchedTradeAccounts:
    return SettleMatchedTradeAccounts(
        key(91), key(92), key(93), key(94), key(95), key(96), key(97), key(98), key(99)
    )


def settle_matched_trade_params() -> SettleMatchedTradeParams:
    return SettleMatchedTradeParams(
        9005,
        1_781_123_456_790,
        fixture_summary(),
        [proof(51, False)],
        [proof(61, True)],
        stat_a(),
        stat_b(),
        market_terms(),
    )


def claim_via_resolution_accounts() -> ClaimViaResolutionAccounts:
    return ClaimViaResolutionAccounts(key(111), key(112), key(113), key(114), key(115), key(116))


def claim_via_resolution_params() -> ClaimViaResolutionParams:
    return ClaimViaResolutionParams(20_615, 17, [proof(70, False), proof(71, True)])


def claim_batch_legacy_accounts() -> ClaimBatchLegacyAccounts:
    return ClaimBatchLegacyAccounts(key(121), key(122), key(123), key(124), key(125))


def claim_batch_legacy_params() -> ClaimBatchLegacyParams:
    return ClaimBatchLegacyParams(
        20_616, 18, hash_bytes(120), True, 941, [proof(72, False), proof(73, True)]
    )


def refund_batch_accounts() -> RefundBatchAccounts:
    return RefundBatchAccounts(key(131), key(132), key(133), key(134))


def audit_trade_result_accounts() -> AuditTradeResultAccounts:
    return AuditTradeResultAccounts(key(141), key(142))


def audit_trade_result_params() -> AuditTradeResultParams:
    terms = MarketIntentParams(
        I32_MAX + 6, 0, 1001, None, TraderPredicate(1, Comparison.GREATER_THAN), None, True
    )
    return AuditTradeResultParams(
        terms,
        fixture_summary(),
        [proof(62, True)],
        [proof(52, False)],
        stat_a(),
        None,
        1_781_123_456_791,
    )


def market_terms() -> MarketIntentParams:
    return MarketIntentParams(
        I32_MAX + 6,
        0,
        1001,
        1002,
        TraderPredicate(1, Comparison.GREATER_THAN),
        BinaryExpression.SUBTRACT,
        False,
    )


def fixture_summary() -> FixtureSummaryInput:
    return FixtureSummaryInput(
        I32_MAX + 6, -3, 1_781_123_456_789, 1_781_123_456_799, hash_bytes(10)
    )


def stat_a() -> StatTermInput:
    return StatTermInput(ScoreStat(1001, 2, 0), hash_bytes(20), [proof(30, True)])


def stat_b() -> StatTermInput:
    return StatTermInput(ScoreStat(1002, -1, 1), hash_bytes(20), [proof(40, False)])


def proof(base: int, is_right_sibling: bool) -> ProofNode:
    return ProofNode(hash32(base), is_right_sibling)


def hash32(base: int) -> Hash32:
    return Hash32(hash_bytes(base))


def hash_bytes(base: int) -> bytes:
    return bytes((base + idx) % 256 for idx in range(32))


def key(base: int) -> Pubkey:
    return Pubkey(bytes([base]) * 32)


I32_MAX = 2**31 - 1


def golden(file_name: str, name: str) -> bytes:
    path = (
        Path(__file__).resolve().parents[2] / "crates" / "txline" / "tests" / "fixtures" / file_name
    )
    data = json.loads(path.read_text())
    for fixture in data["fixtures"]:
        if fixture["name"] == name:
            return bytes.fromhex(fixture["dataHex"])
    raise AssertionError(name)
