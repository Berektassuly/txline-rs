use txline::http::models::UpdateStats;
use txline::validation::legacy::{ScoreStat, ScoresBatchSummary};
use txline::validation::proof::Hash32;
use txline::validation::strategy::{
    BinaryExpression, Comparison, NDimensionalStrategy, TraderPredicate,
};
use txline::validation::v2::{ScoresStatValidationV2, ScoresStatValidationV2Response};
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
