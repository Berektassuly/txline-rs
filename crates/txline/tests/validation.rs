use serde::Deserialize;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use txline::http::models::{
    BatchMetadata, Fixture, FixtureBatchSummary, FixtureBatchValidation, FixtureValidation,
    OddsBatchSummary, OddsPayload, OddsValidation, UpdateStats,
};
use txline::solana::validation::{
    validate_fixture_batch_instruction, validate_fixture_instruction, validate_odds_instruction,
    validate_stat_instruction, validate_stat_v2_instruction,
};
use txline::validation::legacy::{
    FixtureSummaryInput, ScoreStat, ScoresBatchSummary, ScoresStatValidation,
};
use txline::validation::proof::Hash32;
use txline::validation::strategy::{
    BinaryExpression, Comparison, GeometricTarget, NDimensionalStrategy, StatPredicate,
    TraderPredicate,
};
use txline::validation::v2::{
    ScoresStatValidationV2, ScoresStatValidationV2Response, StatLeafInput, StatValidationInput,
};
use txline::{ApiToken, GuestJwt, TxlineClient, TxlineConfig};

#[tokio::test]
async fn rejects_seq_zero_before_auth_or_network() {
    let client = TxlineClient::new(TxlineConfig::devnet()).unwrap();
    let err = client
        .scores()
        .stat_validation_legacy(17952170, 0, 1002, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("seq must be greater than zero"));
}

#[test]
fn v2_preserves_requested_stat_key_order() {
    let validation =
        ScoresStatValidationV2::from_response(vec![1001, 1002], response_with(2)).unwrap();
    assert_eq!(validation.requested_stat_keys(), &[1001, 1002]);
    assert_eq!(validation.stats_to_prove()[0].key, 1001);
    assert_eq!(validation.stats_to_prove()[1].key, 1002);
    assert_eq!(validation.to_validation_input().stats.len(), 2);
}

#[test]
fn v2_rejects_length_mismatch() {
    let err = ScoresStatValidationV2::from_response(vec![1001, 1002, 1007], response_with(2))
        .unwrap_err();
    assert!(err.to_string().contains("statsToProve length"));
}

#[test]
fn v2_rejects_stat_key_order_mismatch() {
    let mut response = response_with(2);
    response.stats_to_prove.swap(0, 1);

    let err = ScoresStatValidationV2::from_response(vec![1001, 1002], response).unwrap_err();
    assert!(err.to_string().contains("statsToProve[0].key"));
    assert!(err.to_string().contains("requested statKeys[0]"));
}

#[test]
fn v2_validation_input_ts_uses_summary_min_timestamp() {
    let response_ts = 1_781_200_000_000;
    let min_timestamp = 1_781_123_456_789;
    let validation = ScoresStatValidationV2::from_response(
        vec![1001, 1002],
        response_with_timestamps(2, response_ts, min_timestamp),
    )
    .unwrap();

    assert_ne!(
        validation.response().ts,
        validation.response().summary.update_stats.min_timestamp
    );
    assert_eq!(validation.target_ts(), min_timestamp);
    assert_eq!(validation.to_validation_input().ts, min_timestamp);
}

#[test]
fn v2_validation_input_preserves_i64_fixture_id() {
    let fixture_id = i64::from(i32::MAX) + 10;
    let validation =
        ScoresStatValidationV2::from_response(vec![1001], response_with_fixture_id(1, fixture_id))
            .unwrap();

    assert_eq!(
        validation.to_validation_input().fixture_summary.fixture_id,
        fixture_id
    );
}

#[test]
fn v2_epoch_day_and_validation_input_ts_use_same_timestamp() {
    let min_timestamp = 20_624_i64 * 86_400_000 + 12_345;
    let response_ts = min_timestamp + 86_400_000;
    let validation = ScoresStatValidationV2::from_response(
        vec![1001],
        response_with_timestamps(1, response_ts, min_timestamp),
    )
    .unwrap();

    let input_epoch_day = (validation.to_validation_input().ts / 86_400_000) as u16;
    assert_eq!(validation.epoch_day().unwrap(), input_epoch_day);
}

#[test]
fn strategy_builder_rejects_out_of_bounds_indices() {
    let predicate = TraderPredicate::new(0, Comparison::equal_to());
    let err = NDimensionalStrategy::builder(2)
        .binary(0, 2, BinaryExpression::subtract(), predicate)
        .unwrap_err();
    assert!(err.to_string().contains("out of bounds"));
}

#[test]
fn strategy_builder_covers_single_binary_geometric_and_multi_leg_shapes() {
    let eq = TraderPredicate::new(0, Comparison::equal_to());
    let gt = TraderPredicate::new(1, Comparison::greater_than());
    let lt = TraderPredicate::new(2, Comparison::less_than());

    let two_leg = NDimensionalStrategy::builder(2)
        .single(0, gt)
        .unwrap()
        .binary(0, 1, BinaryExpression::subtract(), eq)
        .unwrap()
        .geometric_target(0, 0)
        .unwrap()
        .geometric_target(1, 1)
        .unwrap()
        .distance_predicate(lt)
        .build()
        .unwrap();
    assert_eq!(two_leg.discrete_predicates.len(), 2);
    assert_eq!(two_leg.geometric_targets.len(), 2);

    let three_leg = NDimensionalStrategy::builder(3)
        .binary(0, 1, BinaryExpression::subtract(), eq)
        .unwrap()
        .single(2, gt)
        .unwrap()
        .build()
        .unwrap();
    assert_eq!(three_leg.discrete_predicates.len(), 2);

    let four_leg = NDimensionalStrategy::builder(4)
        .binary(0, 1, BinaryExpression::subtract(), gt)
        .unwrap()
        .single(2, eq)
        .unwrap()
        .single(3, lt)
        .unwrap()
        .build()
        .unwrap();
    assert_eq!(four_leg.discrete_predicates.len(), 3);
}

#[test]
fn client_activation_preimage_uses_stored_jwt() {
    let client = TxlineClient::new(TxlineConfig::devnet()).unwrap();
    client.set_guest_jwt(GuestJwt::new("jwt").unwrap());
    client.set_api_token(ApiToken::new("api").unwrap());
    assert_eq!(
        client.activation_preimage("abc", &[1, 2]).unwrap(),
        "abc:1,2:jwt"
    );
}

#[test]
fn validation_instruction_bytes_match_devnet_anchor_golden_fixtures() {
    let program_id = program_id();
    let root = root_account();

    let stat_ix = validate_stat_instruction(
        program_id,
        root,
        &score_validation(),
        TraderPredicate::new(1, Comparison::less_than()),
        Some(BinaryExpression::add()),
    )
    .unwrap();
    assert_eq!(
        stat_ix.accounts,
        vec![AccountMeta::new_readonly(root, false)]
    );
    assert_eq!(stat_ix.data, golden_data("validate_stat"));

    let stat_v2_ix =
        validate_stat_v2_instruction(program_id, root, &stat_v2_payload(), &v2_strategy()).unwrap();
    assert_eq!(
        stat_v2_ix.accounts,
        vec![AccountMeta::new_readonly(root, false)]
    );
    assert_eq!(stat_v2_ix.data, golden_data("validate_stat_v2"));

    let fixture_ix = validate_fixture_instruction(program_id, root, &fixture_validation()).unwrap();
    assert_eq!(
        fixture_ix.accounts,
        vec![AccountMeta::new_readonly(root, false)]
    );
    assert_eq!(fixture_ix.data, golden_data("validate_fixture"));

    let fixture_batch_ix =
        validate_fixture_batch_instruction(program_id, root, 3, &fixture_batch_validation())
            .unwrap();
    assert_eq!(
        fixture_batch_ix.accounts,
        vec![AccountMeta::new_readonly(root, false)]
    );
    assert_eq!(fixture_batch_ix.data, golden_data("validate_fixture_batch"));

    let odds_ix = validate_odds_instruction(program_id, root, &odds_validation()).unwrap();
    assert_eq!(
        odds_ix.accounts,
        vec![AccountMeta::new_readonly(root, false)]
    );
    assert_eq!(odds_ix.data, golden_data("validate_odds"));
}

#[test]
fn score_validation_keeps_signed_update_count_valid() {
    let validation = score_validation();
    assert!(validation.summary.update_stats.update_count < 0);

    let ix = validate_stat_instruction(
        program_id(),
        root_account(),
        &validation,
        TraderPredicate::new(1, Comparison::less_than()),
        Some(BinaryExpression::add()),
    )
    .unwrap();

    assert_eq!(ix.data, golden_data("validate_stat"));
}

#[test]
fn fixture_validation_rejects_negative_update_count() {
    let mut validation = fixture_validation();
    validation.summary.update_stats.update_count = -1;

    let err = validate_fixture_instruction(program_id(), root_account(), &validation).unwrap_err();

    assert!(err.to_string().contains("nonnegative"));
}

#[test]
fn odds_validation_rejects_negative_update_count() {
    let mut validation = odds_validation();
    validation.summary.update_stats.update_count = -1;

    let err = validate_odds_instruction(program_id(), root_account(), &validation).unwrap_err();

    assert!(err.to_string().contains("nonnegative"));
}

fn response_with(count: usize) -> ScoresStatValidationV2Response {
    response_with_timestamps(count, 1, 1)
}

fn response_with_fixture_id(count: usize, fixture_id: i64) -> ScoresStatValidationV2Response {
    let mut response = response_with_timestamps(count, 1, 1);
    response.summary.fixture_id = fixture_id;
    response
}

fn response_with_timestamps(
    count: usize,
    response_ts: i64,
    min_timestamp: i64,
) -> ScoresStatValidationV2Response {
    let hash = Hash32::from_bytes([9u8; 32]).unwrap();
    ScoresStatValidationV2Response {
        ts: response_ts,
        stats_to_prove: (0..count)
            .map(|idx| ScoreStat {
                key: 1001 + idx as u32,
                value: idx as i32,
                period: 0,
            })
            .collect(),
        event_stat_root: hash,
        summary: ScoresBatchSummary {
            fixture_id: 1,
            update_stats: UpdateStats {
                update_count: 1,
                min_timestamp,
                max_timestamp: response_ts.max(min_timestamp),
            },
            event_stats_sub_tree_root: hash,
        },
        stat_proofs: vec![Vec::new(); count],
        sub_tree_proof: Vec::new(),
        main_tree_proof: Vec::new(),
    }
}

fn program_id() -> Pubkey {
    Pubkey::new_from_array([200; 32])
}

fn root_account() -> Pubkey {
    Pubkey::new_from_array([201; 32])
}

fn score_validation() -> ScoresStatValidation {
    let event_stat_root = hash(20);
    ScoresStatValidation {
        ts: 1_781_123_456_789,
        stat_to_prove: ScoreStat {
            key: 1001,
            value: 2,
            period: 0,
        },
        event_stat_root,
        summary: ScoresBatchSummary {
            fixture_id: i64::from(i32::MAX) + 6,
            update_stats: UpdateStats {
                update_count: -3,
                min_timestamp: 1_781_123_456_789,
                max_timestamp: 1_781_123_456_799,
            },
            event_stats_sub_tree_root: hash(10),
        },
        stat_proof: vec![proof(30, true)],
        sub_tree_proof: vec![proof(50, false)],
        main_tree_proof: vec![proof(60, true)],
        stat_to_prove2: Some(ScoreStat {
            key: 1002,
            value: -1,
            period: 1,
        }),
        stat_proof2: Some(vec![proof(40, false)]),
    }
}

fn stat_v2_payload() -> StatValidationInput {
    StatValidationInput {
        ts: 1_781_123_456_789,
        fixture_summary: FixtureSummaryInput {
            fixture_id: i64::from(i32::MAX) + 6,
            update_count: -3,
            min_timestamp: 1_781_123_456_789,
            max_timestamp: 1_781_123_456_799,
            events_sub_tree_root: hash_bytes(10),
        },
        fixture_proof: vec![proof(51, false)],
        main_tree_proof: vec![proof(61, true)],
        event_stat_root: hash_bytes(22),
        stats: vec![
            StatLeafInput {
                stat: ScoreStat {
                    key: 1001,
                    value: 2,
                    period: 0,
                },
                stat_proof: vec![proof(31, true)],
            },
            StatLeafInput {
                stat: ScoreStat {
                    key: 1002,
                    value: -1,
                    period: 1,
                },
                stat_proof: vec![proof(41, false)],
            },
        ],
    }
}

fn v2_strategy() -> NDimensionalStrategy {
    NDimensionalStrategy {
        geometric_targets: vec![
            GeometricTarget {
                stat_index: 0,
                prediction: 0,
            },
            GeometricTarget {
                stat_index: 1,
                prediction: 1,
            },
        ],
        distance_predicate: Some(TraderPredicate::new(2, Comparison::less_than())),
        discrete_predicates: vec![
            StatPredicate::Single {
                index: 0,
                predicate: TraderPredicate::new(1, Comparison::equal_to()),
            },
            StatPredicate::Binary {
                index_a: 0,
                index_b: 1,
                op: BinaryExpression::subtract(),
                predicate: TraderPredicate::new(0, Comparison::greater_than()),
            },
        ],
    }
}

fn fixture_validation() -> FixtureValidation {
    FixtureValidation {
        snapshot: Fixture {
            ts: 1_781_123_000_000,
            start_time: 1_781_126_600_000,
            competition: "Devnet Cup".to_owned(),
            competition_id: 7,
            fixture_group_id: -8,
            participant1_id: 101,
            participant1: "Alpha".to_owned(),
            participant2_id: 202,
            participant2: "Beta".to_owned(),
            fixture_id: i64::from(i32::MAX) + 7,
            participant1_is_home: true,
            extra: Default::default(),
        },
        summary: FixtureBatchSummary {
            fixture_id: i64::from(i32::MAX) + 7,
            competition_id: 7,
            competition: "Devnet Cup".to_owned(),
            update_stats: UpdateStats {
                update_count: 4,
                min_timestamp: 1_781_123_000_000,
                max_timestamp: 1_781_123_000_001,
            },
            update_sub_tree_root: hash(70),
        },
        sub_tree_proof: vec![proof(71, false)],
        main_tree_proof: vec![proof(72, true)],
    }
}

fn fixture_batch_validation() -> FixtureBatchValidation {
    FixtureBatchValidation {
        metadata: BatchMetadata {
            total_update_count: 5,
            num_unique_fixtures: 2,
            overall_batch_start_ts: 1_781_123_000_000,
            overall_batch_end_ts: 1_781_123_900_000,
        },
        proof: vec![proof(80, false), proof(81, true)],
    }
}

fn odds_validation() -> OddsValidation {
    OddsValidation {
        odds: OddsPayload {
            fixture_id: i64::from(i32::MAX) + 8,
            message_id: "msg-1".to_owned(),
            ts: 1_781_123_456_789,
            bookmaker: "Book".to_owned(),
            bookmaker_id: 9,
            super_odds_type: "Winner".to_owned(),
            game_state: Some("PreMatch".to_owned()),
            in_running: false,
            market_parameters: None,
            market_period: Some("FT".to_owned()),
            price_names: vec!["Home".to_owned(), "Away".to_owned()],
            prices: vec![120, -125],
            pct: Vec::new(),
            extra: Default::default(),
        },
        summary: OddsBatchSummary {
            fixture_id: i64::from(i32::MAX) + 8,
            update_stats: UpdateStats {
                update_count: 5,
                min_timestamp: 1_781_123_450_000,
                max_timestamp: 1_781_123_459_999,
            },
            odds_sub_tree_root: hash(90),
        },
        sub_tree_proof: vec![proof(91, false)],
        main_tree_proof: vec![proof(92, true)],
    }
}

fn proof(base: u8, is_right_sibling: bool) -> txline::validation::proof::ProofNode {
    txline::validation::proof::ProofNode {
        hash: hash(base),
        is_right_sibling,
    }
}

fn hash(base: u8) -> Hash32 {
    Hash32::from_bytes(hash_bytes(base)).unwrap()
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
        serde_json::from_str(include_str!("fixtures/validation_golden.devnet.json")).unwrap();
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
