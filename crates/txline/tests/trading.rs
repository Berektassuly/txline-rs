use serde::Deserialize;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use txline::solana::trading::{
    AuditTradeResultAccounts, AuditTradeResultParams, ClaimBatchLegacyAccounts,
    ClaimBatchLegacyParams, ClaimViaResolutionAccounts, ClaimViaResolutionParams,
    CloseIntentAccounts, CloseIntentParams, CreateIntentAccounts, CreateIntentParams,
    CreateTradeAccounts, CreateTradeParams, ExecuteMatchAccounts, ExecuteMatchParams,
    MarketIntentParams, RefundBatchAccounts, RefundBatchParams, SettleMatchedTradeAccounts,
    SettleMatchedTradeParams, SettleTradeAccounts, SettleTradeParams,
    audit_trade_result_instruction, claim_batch_legacy_instruction,
    claim_via_resolution_instruction, close_intent_instruction, create_intent_instruction,
    create_trade_instruction, execute_match_instruction, refund_batch_instruction,
    settle_matched_trade_instruction, settle_trade_instruction,
};
use txline::validation::legacy::{FixtureSummaryInput, ScoreStat, StatTermInput};
use txline::validation::proof::{Hash32, ProofNode};
use txline::validation::strategy::{BinaryExpression, Comparison, TraderPredicate};

#[test]
fn trading_instruction_bytes_match_devnet_anchor_golden_fixtures() {
    let program_id = program_id();

    assert_eq!(
        create_intent_instruction(program_id, create_intent_accounts(), create_intent_params())
            .unwrap()
            .data,
        golden_data("create_intent")
    );
    assert_eq!(
        create_trade_instruction(program_id, create_trade_accounts(), create_trade_params())
            .unwrap()
            .data,
        golden_data("create_trade")
    );
    assert_eq!(
        execute_match_instruction(program_id, execute_match_accounts(), execute_match_params())
            .unwrap()
            .data,
        golden_data("execute_match")
    );
    assert_eq!(
        close_intent_instruction(
            program_id,
            close_intent_accounts(),
            CloseIntentParams::default()
        )
        .unwrap()
        .data,
        golden_data("close_intent")
    );
    assert_eq!(
        settle_trade_instruction(program_id, settle_trade_accounts(), settle_trade_params())
            .unwrap()
            .data,
        golden_data("settle_trade")
    );
    assert_eq!(
        settle_matched_trade_instruction(
            program_id,
            settle_matched_trade_accounts(),
            settle_matched_trade_params()
        )
        .unwrap()
        .data,
        golden_data("settle_matched_trade")
    );
    assert_eq!(
        claim_via_resolution_instruction(
            program_id,
            claim_via_resolution_accounts(),
            claim_via_resolution_params()
        )
        .unwrap()
        .data,
        golden_data("claim_via_resolution")
    );
    assert_eq!(
        claim_batch_legacy_instruction(
            program_id,
            claim_batch_legacy_accounts(),
            claim_batch_legacy_params()
        )
        .unwrap()
        .data,
        golden_data("claim_batch_legacy")
    );
    assert_eq!(
        refund_batch_instruction(
            program_id,
            refund_batch_accounts(),
            RefundBatchParams::default()
        )
        .unwrap()
        .data,
        golden_data("refund_batch")
    );
    assert_eq!(
        audit_trade_result_instruction(
            program_id,
            audit_trade_result_accounts(),
            audit_trade_result_params()
        )
        .unwrap()
        .data,
        golden_data("audit_trade_result")
    );
}

#[test]
fn trading_account_metas_match_devnet_idl_order() {
    let program_id = program_id();

    let create_intent =
        create_intent_instruction(program_id, create_intent_accounts(), create_intent_params())
            .unwrap();
    assert_eq!(
        create_intent.accounts,
        vec![
            writable_signer(1),
            writable(2),
            writable(3),
            writable(4),
            readonly(5),
            readonly(6),
            readonly(7),
            readonly(8),
        ]
    );

    let create_trade =
        create_trade_instruction(program_id, create_trade_accounts(), create_trade_params())
            .unwrap();
    assert_eq!(
        create_trade.accounts,
        vec![
            writable_signer(11),
            writable_signer(12),
            writable_signer(13),
            writable(14),
            writable(15),
            writable(16),
            writable(17),
            readonly(18),
            readonly(19),
            readonly(20),
            readonly(21),
        ]
    );

    let execute_match =
        execute_match_instruction(program_id, execute_match_accounts(), execute_match_params())
            .unwrap();
    assert_eq!(
        execute_match.accounts,
        vec![
            writable_signer(31),
            writable(32),
            writable(33),
            writable(34),
            writable(35),
            writable(36),
            writable(37),
            readonly(38),
            readonly(39),
            readonly(40),
        ]
    );

    let close_intent = close_intent_instruction(
        program_id,
        close_intent_accounts(),
        CloseIntentParams::default(),
    )
    .unwrap();
    assert_eq!(
        close_intent.accounts,
        vec![
            writable(51),
            writable_signer(52),
            writable(53),
            writable(54),
            writable(55),
            readonly(56),
            readonly(57),
            readonly(58),
        ]
    );

    let settle_trade =
        settle_trade_instruction(program_id, settle_trade_accounts(), settle_trade_params())
            .unwrap();
    assert_eq!(
        settle_trade.accounts,
        vec![
            writable_signer(71),
            readonly(72),
            writable(73),
            writable(74),
            writable(75),
            readonly(76),
            readonly(77),
            readonly(78),
            readonly(79),
        ]
    );

    let settle_matched_trade = settle_matched_trade_instruction(
        program_id,
        settle_matched_trade_accounts(),
        settle_matched_trade_params(),
    )
    .unwrap();
    assert_eq!(
        settle_matched_trade.accounts,
        vec![
            writable_signer(91),
            readonly(92),
            writable(93),
            writable(94),
            writable(95),
            readonly(96),
            readonly(97),
            readonly(98),
            readonly(99),
        ]
    );

    let claim_via_resolution = claim_via_resolution_instruction(
        program_id,
        claim_via_resolution_accounts(),
        claim_via_resolution_params(),
    )
    .unwrap();
    assert_eq!(
        claim_via_resolution.accounts,
        vec![
            writable_signer(111),
            readonly(112),
            writable(113),
            writable(114),
            writable(115),
            readonly(116),
        ]
    );

    let claim_batch_legacy = claim_batch_legacy_instruction(
        program_id,
        claim_batch_legacy_accounts(),
        claim_batch_legacy_params(),
    )
    .unwrap();
    assert_eq!(
        claim_batch_legacy.accounts,
        vec![
            writable_signer(121),
            readonly(122),
            readonly(123),
            readonly(124),
            readonly(125),
        ]
    );

    let refund_batch = refund_batch_instruction(
        program_id,
        refund_batch_accounts(),
        RefundBatchParams::default(),
    )
    .unwrap();
    assert_eq!(
        refund_batch.accounts,
        vec![
            writable_signer(131),
            readonly(132),
            readonly(133),
            readonly(134)
        ]
    );

    let audit_trade_result = audit_trade_result_instruction(
        program_id,
        audit_trade_result_accounts(),
        audit_trade_result_params(),
    )
    .unwrap();
    assert_eq!(
        audit_trade_result.accounts,
        vec![writable_signer(141), readonly(142)]
    );
}

fn program_id() -> Pubkey {
    key(200)
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

fn create_intent_params() -> CreateIntentParams {
    CreateIntentParams {
        intent_id: 9001,
        terms_hash: hash_bytes(100),
        deposit_amount: 123_456_789,
        expiration_ts: 1_781_129_999_999,
        claim_period: 42,
        fixture_id: i64::from(i32::MAX) + 6,
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

fn create_trade_params() -> CreateTradeParams {
    CreateTradeParams {
        trade_id: 9002,
        stake_a: 111_111,
        stake_b: 222_222,
        trade_terms_hash: hash_bytes(110),
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

fn execute_match_params() -> ExecuteMatchParams {
    ExecuteMatchParams {
        trade_id: 9003,
        maker_stake: 333_333,
        taker_stake: 444_444,
    }
}

fn close_intent_accounts() -> CloseIntentAccounts {
    CloseIntentAccounts {
        maker: key(51),
        authority: key(52),
        order_intent: key(53),
        intent_vault: key(54),
        maker_token_account: key(55),
        token_mint: key(56),
        token_program: key(57),
        token_treasury_pda: key(58),
    }
}

fn settle_trade_accounts() -> SettleTradeAccounts {
    SettleTradeAccounts {
        winner: key(71),
        daily_scores_merkle_roots: key(72),
        trade_escrow: key(73),
        escrow_vault: key(74),
        winner_token_account: key(75),
        token_mint: key(76),
        token_treasury_pda: key(77),
        token_program: key(78),
        system_program: key(79),
    }
}

fn settle_trade_params() -> SettleTradeParams {
    SettleTradeParams {
        trade_id: 9004,
        ts: 1_781_123_456_789,
        fixture_summary: fixture_summary(),
        fixture_proof: vec![proof(50, false)],
        main_tree_proof: vec![proof(60, true)],
        predicate: TraderPredicate::new(1, Comparison::less_than()),
        stat_a: stat_a(),
        stat_b: Some(stat_b()),
        op: Some(BinaryExpression::add()),
    }
}

fn settle_matched_trade_accounts() -> SettleMatchedTradeAccounts {
    SettleMatchedTradeAccounts {
        winner: key(91),
        daily_scores_merkle_roots: key(92),
        matched_trade: key(93),
        trade_vault: key(94),
        winner_token_account: key(95),
        token_mint: key(96),
        token_treasury_pda: key(97),
        token_program: key(98),
        system_program: key(99),
    }
}

fn settle_matched_trade_params() -> SettleMatchedTradeParams {
    SettleMatchedTradeParams {
        trade_id: 9005,
        ts: 1_781_123_456_790,
        fixture_summary: fixture_summary(),
        fixture_proof: vec![proof(51, false)],
        main_tree_proof: vec![proof(61, true)],
        stat_a: stat_a(),
        stat_b: Some(stat_b()),
        terms: market_terms(),
    }
}

fn claim_via_resolution_accounts() -> ClaimViaResolutionAccounts {
    ClaimViaResolutionAccounts {
        winner: key(111),
        daily_resolution_roots: key(112),
        matched_trade: key(113),
        trade_vault: key(114),
        winner_token_account: key(115),
        token_program: key(116),
    }
}

fn claim_via_resolution_params() -> ClaimViaResolutionParams {
    ClaimViaResolutionParams {
        epoch_day: 20_615,
        interval_index: 17,
        merkle_proof: vec![proof(70, false), proof(71, true)],
    }
}

fn claim_batch_legacy_accounts() -> ClaimBatchLegacyAccounts {
    ClaimBatchLegacyAccounts {
        payer: key(121),
        daily_resolution_roots: key(122),
        token_mint: key(123),
        token_program: key(124),
        system_program: key(125),
    }
}

fn claim_batch_legacy_params() -> ClaimBatchLegacyParams {
    ClaimBatchLegacyParams {
        epoch_day: 20_616,
        interval_index: 18,
        terms_hash: hash_bytes(120),
        winner_is_maker: true,
        seq: 941,
        merkle_proof: vec![proof(72, false), proof(73, true)],
    }
}

fn refund_batch_accounts() -> RefundBatchAccounts {
    RefundBatchAccounts {
        payer: key(131),
        token_mint: key(132),
        token_program: key(133),
        system_program: key(134),
    }
}

fn audit_trade_result_accounts() -> AuditTradeResultAccounts {
    AuditTradeResultAccounts {
        payer: key(141),
        daily_scores_merkle_roots: key(142),
    }
}

fn audit_trade_result_params() -> AuditTradeResultParams {
    let mut terms = market_terms();
    terms.stat_b_key = None;
    terms.op = None;
    terms.negation = true;
    AuditTradeResultParams {
        terms,
        fixture_summary: fixture_summary(),
        main_tree_proof: vec![proof(62, true)],
        fixture_proof: vec![proof(52, false)],
        stat_a: stat_a(),
        stat_b: None,
        ts: 1_781_123_456_791,
    }
}

fn market_terms() -> MarketIntentParams {
    MarketIntentParams {
        fixture_id: i64::from(i32::MAX) + 6,
        period: 0,
        stat_a_key: 1001,
        stat_b_key: Some(1002),
        predicate: TraderPredicate::new(1, Comparison::greater_than()),
        op: Some(BinaryExpression::subtract()),
        negation: false,
    }
}

fn fixture_summary() -> FixtureSummaryInput {
    FixtureSummaryInput {
        fixture_id: i64::from(i32::MAX) + 6,
        update_count: -3,
        min_timestamp: 1_781_123_456_789,
        max_timestamp: 1_781_123_456_799,
        events_sub_tree_root: hash_bytes(10),
    }
}

fn stat_a() -> StatTermInput {
    StatTermInput {
        stat_to_prove: ScoreStat {
            key: 1001,
            value: 2,
            period: 0,
        },
        event_stat_root: hash_bytes(20),
        stat_proof: vec![proof(30, true)],
    }
}

fn stat_b() -> StatTermInput {
    StatTermInput {
        stat_to_prove: ScoreStat {
            key: 1002,
            value: -1,
            period: 1,
        },
        event_stat_root: hash_bytes(20),
        stat_proof: vec![proof(40, false)],
    }
}

fn proof(base: u8, is_right_sibling: bool) -> ProofNode {
    ProofNode {
        hash: Hash32::from_bytes(hash_bytes(base)).unwrap(),
        is_right_sibling,
    }
}

fn writable_signer(base: u8) -> AccountMeta {
    AccountMeta::new(key(base), true)
}

fn writable(base: u8) -> AccountMeta {
    AccountMeta::new(key(base), false)
}

fn readonly(base: u8) -> AccountMeta {
    AccountMeta::new_readonly(key(base), false)
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

fn golden_data(name: &str) -> Vec<u8> {
    let golden: GoldenFile =
        serde_json::from_str(include_str!("fixtures/trading_golden.devnet.json")).unwrap();
    let fixture = golden
        .fixtures
        .iter()
        .find(|fixture| fixture.name == name)
        .unwrap_or_else(|| panic!("missing golden fixture {name}"));
    decode_hex(&fixture.data_hex)
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert!(value.len().is_multiple_of(2));
    (0..value.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&value[offset..offset + 2], 16).unwrap())
        .collect()
}

#[derive(Debug, Deserialize)]
struct GoldenFile {
    fixtures: Vec<GoldenFixture>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoldenFixture {
    name: String,
    data_hex: String,
}
