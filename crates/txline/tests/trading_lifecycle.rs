use std::collections::BTreeMap;

use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use txline::http::models::{Scores, UpdateStats};
use txline::solana::trading::{
    AUDIT_TRADE_RESULT_DISCRIMINATOR, AuditTradeResultAccounts, CLAIM_BATCH_LEGACY_DISCRIMINATOR,
    CLAIM_VIA_RESOLUTION_DISCRIMINATOR, CLOSE_INTENT_DISCRIMINATOR, CREATE_INTENT_DISCRIMINATOR,
    CREATE_TRADE_DISCRIMINATOR, ClaimBatchLegacyAccounts, ClaimBatchLegacyParams,
    ClaimViaResolutionAccounts, ClaimViaResolutionParams, CloseIntentAccounts, CloseIntentParams,
    CreateIntentAccounts, CreateTradeAccounts, EXECUTE_MATCH_DISCRIMINATOR, ExecuteMatchAccounts,
    ExecuteMatchParams, REFUND_BATCH_DISCRIMINATOR, RefundBatchAccounts, RefundBatchParams,
    SETTLE_MATCHED_TRADE_DISCRIMINATOR, SETTLE_TRADE_DISCRIMINATOR, SettleMatchedTradeAccounts,
    SettleTradeAccounts,
};
use txline::solana::validation::VALIDATE_STAT_V2_DISCRIMINATOR;
use txline::validation::legacy::{ScoreStat, ScoresBatchSummary, ScoresStatValidation};
use txline::validation::proof::Hash32;
use txline::validation::strategy::{
    BinaryExpression, Comparison, NDimensionalStrategy, StatPredicate, TraderPredicate,
};
use txline::validation::v2::{ScoresStatValidationV2, ScoresStatValidationV2Response};
use txline::{
    CreateIntentPlanParams, CreateTradePlanParams, FinalOutcomeConfig, LifecycleAction, MarketSide,
    ScoreMarketKind, ScoreMarketTerms, TermsHash, audit_trade_result_params_from_legacy_validation,
    audit_trade_result_plan, claim_batch_legacy_plan, claim_via_resolution_plan, close_intent_plan,
    create_intent_plan, create_trade_plan, execute_match_plan, extract_final_outcome,
    final_outcome_side_strategy, final_outcome_validation_plan, is_final_outcome_record,
    refund_batch_plan, settle_matched_trade_params_from_legacy_validation,
    settle_matched_trade_plan, settle_trade_params_from_legacy_validation, settle_trade_plan,
    validation_input_for_market,
};

#[test]
fn final_outcome_detection_requires_all_documented_markers() {
    let final_score = score_with_stats("game_finalised", Some(100), Some(100), 2, 1);
    assert!(is_final_outcome_record(&final_score));

    let wrong_action = score_with_stats("goal", Some(100), Some(100), 2, 1);
    let wrong_status = score_with_stats("game_finalised", Some(99), Some(100), 2, 1);
    let wrong_period = score_with_stats("game_finalised", Some(100), Some(99), 2, 1);

    assert!(!is_final_outcome_record(&wrong_action));
    assert!(!is_final_outcome_record(&wrong_status));
    assert!(!is_final_outcome_record(&wrong_period));
}

#[test]
fn extract_final_outcome_handles_home_away_and_draw() {
    let cfg = FinalOutcomeConfig::soccer_default();
    let cases = [
        (3, 1, MarketSide::Participant1),
        (0, 2, MarketSide::Participant2),
        (1, 1, MarketSide::Draw),
    ];

    for (participant1_score, participant2_score, expected) in cases {
        let score = score_with_stats(
            "game_finalised",
            Some(100),
            Some(100),
            participant1_score,
            participant2_score,
        );
        let outcome = extract_final_outcome(&score, cfg).unwrap();

        assert_eq!(outcome.result, expected);
        assert_eq!(outcome.participant1_score, participant1_score);
        assert_eq!(outcome.participant2_score, participant2_score);
        assert_eq!(outcome.stat_keys(), [1, 2]);
    }
}

#[test]
fn extract_final_outcome_reports_missing_stats_and_non_final_records() {
    let cfg = FinalOutcomeConfig::soccer_default();
    let mut missing_stats = score_with_stats("game_finalised", Some(100), Some(100), 2, 1);
    missing_stats.stats = None;

    let err = extract_final_outcome(&missing_stats, cfg).unwrap_err();
    assert!(err.to_string().contains("no stats"));

    let mut missing_stat_key = score_with_stats("game_finalised", Some(100), Some(100), 2, 1);
    missing_stat_key.stats.as_mut().unwrap().remove("2");
    let err = extract_final_outcome(&missing_stat_key, cfg).unwrap_err();
    assert!(err.to_string().contains("missing stat key 2"));

    let not_final = score_with_stats("score_update", Some(100), Some(100), 2, 1);
    let err = extract_final_outcome(&not_final, cfg).unwrap_err();
    assert!(err.to_string().contains("not a final outcome record"));
}

#[test]
fn score_market_terms_map_deterministically_to_market_intent_params() {
    let terms = ScoreMarketTerms::final_outcome(
        i64::from(i32::MAX) + 44,
        MarketSide::Participant2,
        FinalOutcomeConfig::soccer_default(),
    );
    let params = terms.to_market_intent_params();

    assert_eq!(terms.kind, ScoreMarketKind::FinalOutcome);
    assert_eq!(terms.stat_keys(), vec![2, 1]);
    assert_eq!(params.fixture_id, i64::from(i32::MAX) + 44);
    assert_eq!(params.period, 100);
    assert_eq!(params.stat_a_key, 2);
    assert_eq!(params.stat_b_key, Some(1));
    assert_eq!(params.op, Some(BinaryExpression::subtract()));
    assert_eq!(
        params.predicate,
        TraderPredicate::new(0, Comparison::greater_than())
    );
    assert!(!params.negation);
}

#[test]
fn explicit_terms_hash_is_stable_and_passed_through() {
    let hash = hash_bytes(7);
    let terms_hash = TermsHash::new(hash);
    let params = CreateIntentPlanParams {
        intent_id: 99,
        terms_hash,
        deposit_amount: 500,
        expiration_ts: 1_781_200_000_000,
        claim_period: 12,
        market: ScoreMarketTerms::final_outcome(
            17_952_170,
            MarketSide::Participant1,
            FinalOutcomeConfig::soccer_default(),
        ),
    };

    assert_eq!(terms_hash.as_bytes(), &hash);
    assert_eq!(terms_hash.into_bytes(), hash);
    assert_eq!(params.to_instruction_params().terms_hash, hash);
}

#[test]
fn final_outcome_strategy_uses_expected_stat_indexes_and_comparisons() {
    assert_binary_strategy(
        final_outcome_side_strategy(MarketSide::Participant1).unwrap(),
        0,
        1,
        Comparison::greater_than(),
    );
    assert_binary_strategy(
        final_outcome_side_strategy(MarketSide::Participant2).unwrap(),
        1,
        0,
        Comparison::greater_than(),
    );
    assert_binary_strategy(
        final_outcome_side_strategy(MarketSide::Draw).unwrap(),
        0,
        1,
        Comparison::equal_to(),
    );
}

#[test]
fn validation_payload_helper_preserves_market_stat_key_order() {
    let terms = ScoreMarketTerms::final_outcome(
        17_952_170,
        MarketSide::Participant2,
        FinalOutcomeConfig::soccer_default(),
    );
    let validation =
        ScoresStatValidationV2::from_response(vec![2, 1], v2_response(17_952_170, &[2, 1]))
            .unwrap();

    let payload = validation_input_for_market(&validation, &terms).unwrap();

    assert_eq!(payload.stats[0].stat.key, 2);
    assert_eq!(payload.stats[1].stat.key, 1);

    let wrong_order =
        ScoresStatValidationV2::from_response(vec![1, 2], v2_response(17_952_170, &[1, 2]))
            .unwrap();
    let err = validation_input_for_market(&wrong_order, &terms).unwrap_err();
    assert!(err.to_string().contains("stat key order"));
}

#[test]
fn legacy_validation_helpers_build_settlement_ready_params() {
    let validation = legacy_validation(17_952_170, 1, Some(2));
    let terms = ScoreMarketTerms::final_outcome(
        17_952_170,
        MarketSide::Participant1,
        FinalOutcomeConfig::soccer_default(),
    );

    let direct = settle_trade_params_from_legacy_validation(88, &validation, &terms).unwrap();
    assert_eq!(direct.trade_id, 88);
    assert_eq!(direct.fixture_summary.fixture_id, 17_952_170);
    assert_eq!(direct.stat_a.stat_to_prove.key, 1);
    assert_eq!(direct.stat_b.unwrap().stat_to_prove.key, 2);
    assert_eq!(direct.op, Some(BinaryExpression::subtract()));

    let matched =
        settle_matched_trade_params_from_legacy_validation(89, &validation, &terms).unwrap();
    assert_eq!(matched.terms.stat_a_key, 1);
    assert_eq!(matched.terms.stat_b_key, Some(2));

    let audit = audit_trade_result_params_from_legacy_validation(&validation, &terms).unwrap();
    assert_eq!(audit.terms.fixture_id, 17_952_170);
}

#[test]
fn lifecycle_plans_use_existing_instruction_builders() {
    let program_id = key(200);
    let terms = ScoreMarketTerms::final_outcome(
        17_952_170,
        MarketSide::Participant1,
        FinalOutcomeConfig::soccer_default(),
    );
    let validation = legacy_validation(17_952_170, 1, Some(2));

    let create_intent = create_intent_plan(
        program_id,
        create_intent_accounts(),
        CreateIntentPlanParams {
            intent_id: 1,
            terms_hash: TermsHash::new(hash_bytes(10)),
            deposit_amount: 100,
            expiration_ts: 1_781_200_000_000,
            claim_period: 4,
            market: terms.clone(),
        },
    )
    .unwrap();
    assert_eq!(create_intent.action, LifecycleAction::CreateIntent);
    assert_eq!(
        &create_intent.instructions[0].data[..8],
        &CREATE_INTENT_DISCRIMINATOR
    );
    assert_eq!(
        create_intent.instructions[0].accounts[0],
        AccountMeta::new(key(1), true)
    );

    let create_trade = create_trade_plan(
        program_id,
        create_trade_accounts(),
        CreateTradePlanParams {
            trade_id: 2,
            stake_a: 100,
            stake_b: 100,
            terms_hash: TermsHash::new(hash_bytes(11)),
        },
    )
    .unwrap();
    assert_eq!(
        &create_trade.instructions[0].data[..8],
        &CREATE_TRADE_DISCRIMINATOR
    );
    assert_eq!(
        create_trade.instructions[0].accounts[1],
        AccountMeta::new(key(12), true)
    );

    let execute_match = execute_match_plan(
        program_id,
        execute_match_accounts(),
        ExecuteMatchParams {
            trade_id: 3,
            maker_stake: 100,
            taker_stake: 100,
        },
    )
    .unwrap();
    assert_eq!(
        &execute_match.instructions[0].data[..8],
        &EXECUTE_MATCH_DISCRIMINATOR
    );

    let close_intent = close_intent_plan(
        program_id,
        close_intent_accounts(),
        CloseIntentParams::default(),
    )
    .unwrap();
    assert_eq!(
        &close_intent.instructions[0].data[..8],
        &CLOSE_INTENT_DISCRIMINATOR
    );

    let settle_trade = settle_trade_plan(
        program_id,
        settle_trade_accounts(),
        settle_trade_params_from_legacy_validation(4, &validation, &terms).unwrap(),
    )
    .unwrap();
    assert_eq!(
        &settle_trade.instructions[0].data[..8],
        &SETTLE_TRADE_DISCRIMINATOR
    );

    let settle_matched = settle_matched_trade_plan(
        program_id,
        settle_matched_trade_accounts(),
        settle_matched_trade_params_from_legacy_validation(5, &validation, &terms).unwrap(),
    )
    .unwrap();
    assert_eq!(
        &settle_matched.instructions[0].data[..8],
        &SETTLE_MATCHED_TRADE_DISCRIMINATOR
    );

    let claim_resolution = claim_via_resolution_plan(
        program_id,
        claim_via_resolution_accounts(),
        ClaimViaResolutionParams {
            epoch_day: 20_615,
            interval_index: 1,
            merkle_proof: Vec::new(),
        },
    )
    .unwrap();
    assert_eq!(
        &claim_resolution.instructions[0].data[..8],
        &CLAIM_VIA_RESOLUTION_DISCRIMINATOR
    );

    let claim_legacy = claim_batch_legacy_plan(
        program_id,
        claim_batch_legacy_accounts(),
        ClaimBatchLegacyParams {
            epoch_day: 20_615,
            interval_index: 1,
            terms_hash: hash_bytes(12),
            winner_is_maker: true,
            seq: 941,
            merkle_proof: Vec::new(),
        },
    )
    .unwrap();
    assert_eq!(
        &claim_legacy.instructions[0].data[..8],
        &CLAIM_BATCH_LEGACY_DISCRIMINATOR
    );

    let refund = refund_batch_plan(
        program_id,
        RefundBatchAccounts {
            payer: key(61),
            token_mint: key(62),
            token_program: key(63),
            system_program: key(64),
        },
        RefundBatchParams::default(),
    )
    .unwrap();
    assert_eq!(
        &refund.instructions[0].data[..8],
        &REFUND_BATCH_DISCRIMINATOR
    );

    let audit = audit_trade_result_plan(
        program_id,
        AuditTradeResultAccounts {
            payer: key(71),
            daily_scores_merkle_roots: key(72),
        },
        audit_trade_result_params_from_legacy_validation(&validation, &terms).unwrap(),
    )
    .unwrap();
    assert_eq!(
        &audit.instructions[0].data[..8],
        &AUDIT_TRADE_RESULT_DISCRIMINATOR
    );
}

#[test]
fn final_outcome_validation_plan_builds_v2_validation_instruction() {
    let score = score_with_stats("game_finalised", Some(100), Some(100), 2, 1);
    let outcome = extract_final_outcome(&score, FinalOutcomeConfig::soccer_default()).unwrap();
    let validation =
        ScoresStatValidationV2::from_response(vec![1, 2], v2_response(score.fixture_id, &[1, 2]))
            .unwrap();

    let plan = final_outcome_validation_plan(key(200), &validation, &outcome).unwrap();

    assert_eq!(plan.action, LifecycleAction::ValidateFinalOutcome);
    assert_eq!(plan.instructions.len(), 1);
    assert_eq!(
        &plan.instructions[0].data[..8],
        &VALIDATE_STAT_V2_DISCRIMINATOR
    );
    assert_eq!(plan.instructions[0].accounts.len(), 1);
    assert!(!plan.instructions[0].accounts[0].is_writable);
    assert!(!plan.instructions[0].accounts[0].is_signer);
}

fn assert_binary_strategy(
    strategy: NDimensionalStrategy,
    expected_a: u8,
    expected_b: u8,
    expected_comparison: Comparison,
) {
    assert_eq!(strategy.discrete_predicates.len(), 1);
    match strategy.discrete_predicates[0] {
        StatPredicate::Binary {
            index_a,
            index_b,
            op,
            predicate,
        } => {
            assert_eq!(index_a, expected_a);
            assert_eq!(index_b, expected_b);
            assert_eq!(op, BinaryExpression::subtract());
            assert_eq!(predicate, TraderPredicate::new(0, expected_comparison));
        }
        other => panic!("expected binary predicate, got {other:?}"),
    }
}

fn score_with_stats(
    action: &str,
    status_id: Option<i32>,
    period: Option<i32>,
    participant1_score: i32,
    participant2_score: i32,
) -> Scores {
    let mut stats = BTreeMap::new();
    stats.insert("1".to_owned(), participant1_score);
    stats.insert("2".to_owned(), participant2_score);
    Scores {
        fixture_id: 17_952_170,
        game_state: "final".to_owned(),
        start_time: 1_781_100_000_000,
        is_team: true,
        fixture_group_id: 10,
        competition_id: 20,
        country_id: 30,
        sport_id: 1,
        participant1_is_home: true,
        participant2_id: 202,
        participant1_id: 101,
        action: action.to_owned(),
        id: 99,
        ts: 1_781_200_000_000,
        connection_id: 55,
        seq: 941,
        status_id,
        period,
        coverage_secondary_data: None,
        coverage_type: None,
        confirmed: None,
        participant: None,
        possession: None,
        stats: Some(stats),
        player_stats: None,
        extra: Default::default(),
    }
}

fn v2_response(fixture_id: i64, keys: &[u32]) -> ScoresStatValidationV2Response {
    let hash = Hash32::from_bytes(hash_bytes(30)).unwrap();
    ScoresStatValidationV2Response {
        ts: 86_400_001,
        stats_to_prove: keys
            .iter()
            .enumerate()
            .map(|(idx, key)| ScoreStat {
                key: *key,
                value: idx as i32,
                period: 100,
            })
            .collect(),
        event_stat_root: hash,
        summary: ScoresBatchSummary {
            fixture_id,
            update_stats: UpdateStats {
                update_count: 1,
                min_timestamp: 86_400_000,
                max_timestamp: 86_400_001,
            },
            event_stats_sub_tree_root: hash,
        },
        stat_proofs: vec![Vec::new(); keys.len()],
        sub_tree_proof: Vec::new(),
        main_tree_proof: Vec::new(),
    }
}

fn legacy_validation(
    fixture_id: i64,
    stat_a_key: u32,
    stat_b_key: Option<u32>,
) -> ScoresStatValidation {
    let hash = Hash32::from_bytes(hash_bytes(40)).unwrap();
    ScoresStatValidation {
        ts: 86_400_001,
        stat_to_prove: ScoreStat {
            key: stat_a_key,
            value: 2,
            period: 100,
        },
        event_stat_root: hash,
        summary: ScoresBatchSummary {
            fixture_id,
            update_stats: UpdateStats {
                update_count: 1,
                min_timestamp: 86_400_000,
                max_timestamp: 86_400_001,
            },
            event_stats_sub_tree_root: hash,
        },
        stat_proof: Vec::new(),
        sub_tree_proof: Vec::new(),
        main_tree_proof: Vec::new(),
        stat_to_prove2: stat_b_key.map(|key| ScoreStat {
            key,
            value: 1,
            period: 100,
        }),
        stat_proof2: stat_b_key.map(|_| Vec::new()),
    }
}

fn create_intent_accounts() -> CreateIntentAccounts {
    CreateIntentAccounts {
        maker: key(1),
        order_intent: key(2),
        intent_vault: key(3),
        maker_token_account: key(4),
        token_mint: key(5),
        token_treasury_pda: key(6),
        token_program: key(7),
        system_program: key(8),
    }
}

fn create_trade_accounts() -> CreateTradeAccounts {
    CreateTradeAccounts {
        authority: key(11),
        trader_a: key(12),
        trader_b: key(13),
        trader_a_token_account: key(14),
        trader_b_token_account: key(15),
        trade_escrow: key(16),
        escrow_vault: key(17),
        stake_token_mint: key(18),
        token_treasury_pda: key(19),
        token_program: key(20),
        system_program: key(21),
    }
}

fn execute_match_accounts() -> ExecuteMatchAccounts {
    ExecuteMatchAccounts {
        solver: key(31),
        maker_intent: key(32),
        taker_intent: key(33),
        maker_vault: key(34),
        taker_vault: key(35),
        matched_trade: key(36),
        trade_vault: key(37),
        token_mint: key(38),
        token_program: key(39),
        system_program: key(40),
    }
}

fn close_intent_accounts() -> CloseIntentAccounts {
    CloseIntentAccounts {
        maker: key(41),
        authority: key(42),
        order_intent: key(43),
        intent_vault: key(44),
        maker_token_account: key(45),
        token_mint: key(46),
        token_program: key(47),
        token_treasury_pda: key(48),
    }
}

fn settle_trade_accounts() -> SettleTradeAccounts {
    SettleTradeAccounts {
        winner: key(51),
        daily_scores_merkle_roots: key(52),
        trade_escrow: key(53),
        escrow_vault: key(54),
        winner_token_account: key(55),
        token_mint: key(56),
        token_treasury_pda: key(57),
        token_program: key(58),
        system_program: key(59),
    }
}

fn settle_matched_trade_accounts() -> SettleMatchedTradeAccounts {
    SettleMatchedTradeAccounts {
        winner: key(81),
        daily_scores_merkle_roots: key(82),
        matched_trade: key(83),
        trade_vault: key(84),
        winner_token_account: key(85),
        token_mint: key(86),
        token_treasury_pda: key(87),
        token_program: key(88),
        system_program: key(89),
    }
}

fn claim_via_resolution_accounts() -> ClaimViaResolutionAccounts {
    ClaimViaResolutionAccounts {
        winner: key(91),
        daily_resolution_roots: key(92),
        matched_trade: key(93),
        trade_vault: key(94),
        winner_token_account: key(95),
        token_program: key(96),
    }
}

fn claim_batch_legacy_accounts() -> ClaimBatchLegacyAccounts {
    ClaimBatchLegacyAccounts {
        payer: key(101),
        daily_resolution_roots: key(102),
        token_mint: key(103),
        token_program: key(104),
        system_program: key(105),
    }
}

fn key(base: u8) -> Pubkey {
    Pubkey::new_from_array([base; 32])
}

fn hash_bytes(base: u8) -> [u8; 32] {
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = base.wrapping_add(index as u8);
    }
    bytes
}
