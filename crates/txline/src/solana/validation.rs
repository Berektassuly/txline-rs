//! Devnet on-chain validation instruction builders and simulation helpers.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;

use super::codec::{
    encode_binary_expression, encode_option, encode_proof_vec, encode_score_stat,
    encode_scores_batch_summary, encode_stat_term, encode_string_option, encode_trader_predicate,
    put_bool, put_i32, put_i64, put_string, put_u8, put_u32, put_vec,
};
use super::pda::{COMPUTE_BUDGET_PROGRAM_ID, DevnetPdas, parse_pubkey};
use crate::config::TxlineConfig;
use crate::http::models::{
    BatchMetadata, Fixture, FixtureBatchSummary, FixtureBatchValidation, FixtureValidation,
    OddsBatchSummary, OddsPayload, OddsValidation, UpdateStats,
};
use crate::validation::legacy::{ScoresStatValidation, timestamp_ms_to_epoch_day};
use crate::validation::strategy::{
    BinaryExpression, NDimensionalStrategy, StatPredicate, TraderPredicate,
};
use crate::validation::v2::{StatLeafInput, StatValidationInput};
use crate::{Result, TxlineError};

pub const VALIDATE_FIXTURE_DISCRIMINATOR: [u8; 8] = [231, 129, 218, 86, 223, 114, 21, 126];
pub const VALIDATE_FIXTURE_BATCH_DISCRIMINATOR: [u8; 8] = [85, 223, 204, 7, 4, 87, 157, 1];
pub const VALIDATE_ODDS_DISCRIMINATOR: [u8; 8] = [192, 19, 91, 138, 104, 100, 212, 86];
pub const VALIDATE_STAT_DISCRIMINATOR: [u8; 8] = [107, 197, 232, 90, 191, 136, 105, 185];
pub const VALIDATE_STAT_V2_DISCRIMINATOR: [u8; 8] = [208, 215, 194, 214, 241, 71, 246, 178];

pub const DEFAULT_VALIDATION_COMPUTE_UNITS: u32 = 1_400_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationSimulationConfig {
    pub compute_unit_limit: u32,
}

impl Default for ValidationSimulationConfig {
    fn default() -> Self {
        Self {
            compute_unit_limit: DEFAULT_VALIDATION_COMPUTE_UNITS,
        }
    }
}

pub fn validate_stat_instruction(
    program_id: Pubkey,
    daily_scores_merkle_roots: Pubkey,
    validation: &ScoresStatValidation,
    predicate: TraderPredicate,
    op: Option<BinaryExpression>,
) -> Result<Instruction> {
    let stat_a = validation.primary_stat_term();
    let stat_b = validation.secondary_stat_term()?;
    let target_ts = validation.summary.update_stats.min_timestamp;

    let mut data = Vec::new();
    data.extend_from_slice(&VALIDATE_STAT_DISCRIMINATOR);
    put_i64(&mut data, target_ts);
    encode_scores_batch_summary(&mut data, &validation.fixture_summary_input());
    encode_proof_vec(&mut data, &validation.sub_tree_proof)?;
    encode_proof_vec(&mut data, &validation.main_tree_proof)?;
    encode_trader_predicate(&mut data, &predicate);
    encode_stat_term(&mut data, &stat_a)?;
    encode_option(&mut data, stat_b.as_ref(), encode_stat_term)?;
    encode_option(&mut data, op.as_ref(), |out, op| {
        encode_binary_expression(out, op);
        Ok(())
    })?;

    Ok(Instruction {
        program_id,
        accounts: vec![AccountMeta::new_readonly(daily_scores_merkle_roots, false)],
        data,
    })
}

pub fn devnet_validate_stat_instruction(
    program_id: Pubkey,
    validation: &ScoresStatValidation,
    predicate: TraderPredicate,
    op: Option<BinaryExpression>,
) -> Result<Instruction> {
    let pdas = DevnetPdas::new()?;
    let root = pdas.daily_scores_roots(validation.epoch_day()?).address;
    validate_stat_instruction(program_id, root, validation, predicate, op)
}

pub fn validate_stat_v2_instruction(
    program_id: Pubkey,
    daily_scores_merkle_roots: Pubkey,
    payload: &StatValidationInput,
    strategy: &NDimensionalStrategy,
) -> Result<Instruction> {
    strategy.validate_indices(payload.stats.len())?;
    let mut data = Vec::new();
    data.extend_from_slice(&VALIDATE_STAT_V2_DISCRIMINATOR);
    encode_stat_validation_input(&mut data, payload)?;
    encode_ndimensional_strategy(&mut data, strategy)?;

    Ok(Instruction {
        program_id,
        accounts: vec![AccountMeta::new_readonly(daily_scores_merkle_roots, false)],
        data,
    })
}

pub fn devnet_validate_stat_v2_instruction(
    program_id: Pubkey,
    payload: &StatValidationInput,
    strategy: &NDimensionalStrategy,
) -> Result<Instruction> {
    let pdas = DevnetPdas::new()?;
    let epoch_day = timestamp_ms_to_epoch_day(payload.ts)?;
    let root = pdas.daily_scores_roots(epoch_day).address;
    validate_stat_v2_instruction(program_id, root, payload, strategy)
}

pub fn validate_fixture_instruction(
    program_id: Pubkey,
    ten_daily_fixtures_roots: Pubkey,
    validation: &FixtureValidation,
) -> Result<Instruction> {
    let mut data = Vec::new();
    data.extend_from_slice(&VALIDATE_FIXTURE_DISCRIMINATOR);
    encode_fixture(&mut data, &validation.snapshot)?;
    encode_fixture_batch_summary(&mut data, &validation.summary)?;
    encode_proof_vec(&mut data, &validation.sub_tree_proof)?;
    encode_proof_vec(&mut data, &validation.main_tree_proof)?;

    Ok(Instruction {
        program_id,
        accounts: vec![AccountMeta::new_readonly(ten_daily_fixtures_roots, false)],
        data,
    })
}

pub fn devnet_validate_fixture_instruction(
    program_id: Pubkey,
    validation: &FixtureValidation,
) -> Result<Instruction> {
    let pdas = DevnetPdas::new()?;
    let epoch_day = timestamp_ms_to_epoch_day(validation.summary.update_stats.min_timestamp)?;
    let root = pdas.ten_daily_fixtures_roots(epoch_day).address;
    validate_fixture_instruction(program_id, root, validation)
}

pub fn validate_fixture_batch_instruction(
    program_id: Pubkey,
    ten_daily_fixtures_roots: Pubkey,
    index: u8,
    validation: &FixtureBatchValidation,
) -> Result<Instruction> {
    let mut data = Vec::new();
    data.extend_from_slice(&VALIDATE_FIXTURE_BATCH_DISCRIMINATOR);
    put_u8(&mut data, index);
    encode_batch_metadata(&mut data, &validation.metadata);
    encode_proof_vec(&mut data, &validation.proof)?;

    Ok(Instruction {
        program_id,
        accounts: vec![AccountMeta::new_readonly(ten_daily_fixtures_roots, false)],
        data,
    })
}

pub fn devnet_validate_fixture_batch_instruction(
    program_id: Pubkey,
    epoch_day: u16,
    index: u8,
    validation: &FixtureBatchValidation,
) -> Result<Instruction> {
    let pdas = DevnetPdas::new()?;
    let root = pdas.ten_daily_fixtures_roots(epoch_day).address;
    validate_fixture_batch_instruction(program_id, root, index, validation)
}

pub fn validate_odds_instruction(
    program_id: Pubkey,
    daily_odds_merkle_roots: Pubkey,
    validation: &OddsValidation,
) -> Result<Instruction> {
    let mut data = Vec::new();
    data.extend_from_slice(&VALIDATE_ODDS_DISCRIMINATOR);
    put_i64(&mut data, validation.odds.ts);
    encode_odds(&mut data, &validation.odds)?;
    encode_odds_batch_summary(&mut data, &validation.summary)?;
    encode_proof_vec(&mut data, &validation.sub_tree_proof)?;
    encode_proof_vec(&mut data, &validation.main_tree_proof)?;

    Ok(Instruction {
        program_id,
        accounts: vec![AccountMeta::new_readonly(daily_odds_merkle_roots, false)],
        data,
    })
}

pub fn devnet_validate_odds_instruction(
    program_id: Pubkey,
    validation: &OddsValidation,
) -> Result<Instruction> {
    let pdas = DevnetPdas::new()?;
    let epoch_day = timestamp_ms_to_epoch_day(validation.summary.update_stats.min_timestamp)?;
    let root = pdas.daily_odds_merkle_roots(epoch_day).address;
    validate_odds_instruction(program_id, root, validation)
}

pub fn simulate_validation_instruction<S: Signer>(
    config: &TxlineConfig,
    payer: &S,
    instruction: Instruction,
    simulation_config: ValidationSimulationConfig,
) -> Result<bool> {
    let rpc = RpcClient::new(config.rpc_url.clone());
    let blockhash = rpc.get_latest_blockhash()?;
    let mut instructions = vec![compute_unit_limit_instruction(
        simulation_config.compute_unit_limit,
    )?];
    instructions.push(instruction);
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    transaction.sign(&[payer], blockhash);

    let result = rpc.simulate_transaction(&transaction)?.value;
    if let Some(err) = result.err {
        return Err(TxlineError::solana(format!(
            "validation simulation failed: {err:?}"
        )));
    }
    decode_return_bool(config, result.return_data)
}

pub(crate) fn decode_return_bool(
    config: &TxlineConfig,
    return_data: Option<solana_client::rpc_response::UiTransactionReturnData>,
) -> Result<bool> {
    let return_data =
        return_data.ok_or_else(|| TxlineError::solana("validation simulation returned no data"))?;
    if return_data.program_id != config.program_id {
        return Err(TxlineError::solana(format!(
            "validation simulation returned data for unexpected program {}",
            return_data.program_id
        )));
    }
    let bytes = STANDARD.decode(return_data.data.0)?;
    match bytes.as_slice() {
        [0] => Ok(false),
        [1] => Ok(true),
        _ => Err(TxlineError::solana(format!(
            "validation simulation returned {} bytes, expected one Borsh bool byte",
            bytes.len()
        ))),
    }
}

pub(crate) fn compute_unit_limit_instruction(units: u32) -> Result<Instruction> {
    let mut data = Vec::with_capacity(5);
    data.push(2);
    data.extend_from_slice(&units.to_le_bytes());
    Ok(Instruction {
        program_id: parse_pubkey(COMPUTE_BUDGET_PROGRAM_ID)?,
        accounts: Vec::new(),
        data,
    })
}

fn encode_stat_validation_input(out: &mut Vec<u8>, input: &StatValidationInput) -> Result<()> {
    put_i64(out, input.ts);
    encode_scores_batch_summary(out, &input.fixture_summary);
    encode_proof_vec(out, &input.fixture_proof)?;
    encode_proof_vec(out, &input.main_tree_proof)?;
    out.extend_from_slice(&input.event_stat_root);
    put_vec(out, &input.stats, encode_stat_leaf)
}

fn encode_stat_leaf(out: &mut Vec<u8>, leaf: &StatLeafInput) -> Result<()> {
    encode_score_stat(out, &leaf.stat);
    encode_proof_vec(out, &leaf.stat_proof)
}

fn encode_ndimensional_strategy(out: &mut Vec<u8>, strategy: &NDimensionalStrategy) -> Result<()> {
    put_vec(out, &strategy.geometric_targets, |out, target| {
        put_u8(out, target.stat_index);
        put_i32(out, target.prediction);
        Ok(())
    })?;
    encode_option(
        out,
        strategy.distance_predicate.as_ref(),
        |out, predicate| {
            encode_trader_predicate(out, predicate);
            Ok(())
        },
    )?;
    put_vec(out, &strategy.discrete_predicates, encode_stat_predicate)
}

fn encode_stat_predicate(out: &mut Vec<u8>, predicate: &StatPredicate) -> Result<()> {
    match predicate {
        StatPredicate::Single { index, predicate } => {
            put_u8(out, 0);
            put_u8(out, *index);
            encode_trader_predicate(out, predicate);
        }
        StatPredicate::Binary {
            index_a,
            index_b,
            op,
            predicate,
        } => {
            put_u8(out, 1);
            put_u8(out, *index_a);
            put_u8(out, *index_b);
            encode_binary_expression(out, op);
            encode_trader_predicate(out, predicate);
        }
    }
    Ok(())
}

fn encode_fixture(out: &mut Vec<u8>, fixture: &Fixture) -> Result<()> {
    put_i64(out, fixture.ts);
    put_i64(out, fixture.start_time);
    put_string(out, &fixture.competition)?;
    put_i32(out, fixture.competition_id);
    put_i32(out, fixture.fixture_group_id);
    put_i32(out, fixture.participant1_id);
    put_string(out, &fixture.participant1)?;
    put_i32(out, fixture.participant2_id);
    put_string(out, &fixture.participant2)?;
    put_i64(out, fixture.fixture_id);
    put_bool(out, fixture.participant1_is_home);
    Ok(())
}

fn encode_fixture_batch_summary(out: &mut Vec<u8>, summary: &FixtureBatchSummary) -> Result<()> {
    put_i64(out, summary.fixture_id);
    put_i32(out, summary.competition_id);
    put_string(out, &summary.competition)?;
    encode_update_stats_u32(out, &summary.update_stats)?;
    out.extend_from_slice(summary.update_sub_tree_root.as_bytes());
    Ok(())
}

fn encode_batch_metadata(out: &mut Vec<u8>, metadata: &BatchMetadata) {
    put_i32(out, metadata.total_update_count);
    put_i32(out, metadata.num_unique_fixtures);
    put_i64(out, metadata.overall_batch_start_ts);
    put_i64(out, metadata.overall_batch_end_ts);
}

fn encode_odds(out: &mut Vec<u8>, odds: &OddsPayload) -> Result<()> {
    put_i64(out, odds.fixture_id);
    put_string(out, &odds.message_id)?;
    put_i64(out, odds.ts);
    put_string(out, &odds.bookmaker)?;
    put_i32(out, odds.bookmaker_id);
    put_string(out, &odds.super_odds_type)?;
    encode_string_option(out, odds.game_state.as_deref())?;
    put_bool(out, odds.in_running);
    encode_string_option(out, odds.market_parameters.as_deref())?;
    encode_string_option(out, odds.market_period.as_deref())?;
    put_vec(out, &odds.price_names, |out, value| put_string(out, value))?;
    put_vec(out, &odds.prices, |out, value| {
        put_i32(out, *value);
        Ok(())
    })
}

fn encode_odds_batch_summary(out: &mut Vec<u8>, summary: &OddsBatchSummary) -> Result<()> {
    put_i64(out, summary.fixture_id);
    encode_update_stats_u32(out, &summary.update_stats)?;
    out.extend_from_slice(summary.odds_sub_tree_root.as_bytes());
    Ok(())
}

fn encode_update_stats_u32(out: &mut Vec<u8>, update_stats: &UpdateStats) -> Result<()> {
    put_u32(
        out,
        nonnegative_u32(update_stats.update_count, "update_count")?,
    );
    put_i64(out, update_stats.min_timestamp);
    put_i64(out, update_stats.max_timestamp);
    Ok(())
}

fn nonnegative_u32(value: i32, name: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| {
        TxlineError::validation(format!(
            "{name} must be nonnegative to match the Devnet IDL u32 field"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::legacy::{FixtureSummaryInput, ScoreStat};
    use crate::validation::proof::Hash32;
    use crate::validation::strategy::Comparison;

    #[test]
    fn validate_stat_encodes_fixture_id_as_i64() {
        let program_id = Pubkey::new_unique();
        let root = Pubkey::new_unique();
        let hash = Hash32::from_bytes([7u8; 32]).unwrap();
        let validation = ScoresStatValidation {
            ts: 1,
            stat_to_prove: ScoreStat {
                key: 1001,
                value: 2,
                period: 0,
            },
            event_stat_root: hash,
            summary: crate::validation::legacy::ScoresBatchSummary {
                fixture_id: i64::from(i32::MAX) + 1,
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
            stat_to_prove2: None,
            stat_proof2: None,
        };
        let ix = validate_stat_instruction(
            program_id,
            root,
            &validation,
            TraderPredicate::new(0, Comparison::greater_than()),
            None,
        )
        .unwrap();

        let fixture_id_offset = VALIDATE_STAT_DISCRIMINATOR.len() + 8;
        assert_eq!(
            &ix.data[fixture_id_offset..fixture_id_offset + 8],
            &(i64::from(i32::MAX) + 1).to_le_bytes()
        );
    }

    #[test]
    fn compute_budget_limit_instruction_matches_solana_layout() {
        let ix = compute_unit_limit_instruction(1_400_000).unwrap();
        assert_eq!(ix.accounts.len(), 0);
        assert_eq!(ix.data, [2, 192, 92, 21, 0]);
    }

    #[test]
    fn validate_stat_v2_uses_pr_discriminator() {
        let program_id = Pubkey::new_unique();
        let root = Pubkey::new_unique();
        let payload = StatValidationInput {
            ts: 86_400_000,
            fixture_summary: FixtureSummaryInput {
                fixture_id: i64::from(i32::MAX) + 2,
                update_count: 1,
                min_timestamp: 86_400_000,
                max_timestamp: 86_400_001,
                events_sub_tree_root: [8u8; 32],
            },
            fixture_proof: Vec::new(),
            main_tree_proof: Vec::new(),
            event_stat_root: [9u8; 32],
            stats: Vec::new(),
        };
        let strategy = NDimensionalStrategy::builder(0).build().unwrap();

        let ix = validate_stat_v2_instruction(program_id, root, &payload, &strategy).unwrap();

        assert_eq!(&ix.data[..8], &VALIDATE_STAT_V2_DISCRIMINATOR);
        assert_eq!(ix.accounts, vec![AccountMeta::new_readonly(root, false)]);
    }

    #[test]
    fn validate_fixture_and_odds_use_expected_discriminators() {
        let program_id = Pubkey::new_unique();
        let root = Pubkey::new_unique();
        let hash = Hash32::from_bytes([4u8; 32]).unwrap();
        let update_stats = UpdateStats {
            update_count: 1,
            min_timestamp: 86_400_000,
            max_timestamp: 86_400_001,
        };
        let fixture_validation = FixtureValidation {
            snapshot: Fixture {
                ts: 86_400_000,
                start_time: 86_500_000,
                competition: "Cup".to_owned(),
                competition_id: 1,
                fixture_group_id: 2,
                participant1_id: 3,
                participant1: "A".to_owned(),
                participant2_id: 4,
                participant2: "B".to_owned(),
                fixture_id: 5,
                participant1_is_home: true,
                extra: Default::default(),
            },
            summary: FixtureBatchSummary {
                fixture_id: 5,
                competition_id: 1,
                competition: "Cup".to_owned(),
                update_stats: update_stats.clone(),
                update_sub_tree_root: hash,
            },
            sub_tree_proof: Vec::new(),
            main_tree_proof: Vec::new(),
        };
        let odds_validation = OddsValidation {
            odds: OddsPayload {
                fixture_id: 5,
                message_id: "message".to_owned(),
                ts: 86_400_000,
                bookmaker: "Book".to_owned(),
                bookmaker_id: 1,
                super_odds_type: "Winner".to_owned(),
                game_state: None,
                in_running: false,
                market_parameters: None,
                market_period: None,
                price_names: vec!["Home".to_owned()],
                prices: vec![100],
                pct: Vec::new(),
                extra: Default::default(),
            },
            summary: OddsBatchSummary {
                fixture_id: 5,
                update_stats,
                odds_sub_tree_root: hash,
            },
            sub_tree_proof: Vec::new(),
            main_tree_proof: Vec::new(),
        };

        let fixture_ix =
            validate_fixture_instruction(program_id, root, &fixture_validation).unwrap();
        let odds_ix = validate_odds_instruction(program_id, root, &odds_validation).unwrap();

        assert_eq!(&fixture_ix.data[..8], &VALIDATE_FIXTURE_DISCRIMINATOR);
        assert_eq!(&odds_ix.data[..8], &VALIDATE_ODDS_DISCRIMINATOR);
    }
}
