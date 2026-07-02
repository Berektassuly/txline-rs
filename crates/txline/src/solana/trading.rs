//! Low-level Devnet TxODDS public trading instruction builders.

use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;

use super::codec::{
    encode_binary_expression, encode_option, encode_proof_vec, encode_scores_batch_summary,
    encode_stat_term, encode_trader_predicate, put_bool, put_i64, put_u16, put_u32, put_u64,
};
use crate::Result;
use crate::validation::legacy::{FixtureSummaryInput, StatTermInput};
use crate::validation::proof::ProofNode;
use crate::validation::strategy::{BinaryExpression, TraderPredicate};

pub const CREATE_INTENT_DISCRIMINATOR: [u8; 8] = [216, 214, 79, 121, 23, 194, 96, 104];
pub const CREATE_TRADE_DISCRIMINATOR: [u8; 8] = [183, 82, 24, 245, 248, 30, 204, 246];
pub const EXECUTE_MATCH_DISCRIMINATOR: [u8; 8] = [76, 47, 91, 223, 20, 10, 147, 232];
pub const CLOSE_INTENT_DISCRIMINATOR: [u8; 8] = [112, 245, 154, 249, 57, 126, 54, 122];
pub const SETTLE_TRADE_DISCRIMINATOR: [u8; 8] = [252, 176, 98, 248, 73, 123, 8, 157];
pub const SETTLE_MATCHED_TRADE_DISCRIMINATOR: [u8; 8] = [191, 233, 149, 116, 32, 239, 18, 65];
pub const CLAIM_VIA_RESOLUTION_DISCRIMINATOR: [u8; 8] = [98, 206, 250, 87, 151, 135, 162, 181];
pub const CLAIM_BATCH_LEGACY_DISCRIMINATOR: [u8; 8] = [254, 101, 89, 255, 169, 75, 207, 66];
pub const REFUND_BATCH_DISCRIMINATOR: [u8; 8] = [227, 54, 194, 2, 78, 8, 104, 29];
pub const AUDIT_TRADE_RESULT_DISCRIMINATOR: [u8; 8] = [50, 242, 243, 5, 209, 75, 76, 91];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateIntentAccounts {
    pub maker: Pubkey,
    pub order_intent: Pubkey,
    pub intent_vault: Pubkey,
    pub maker_token_account: Pubkey,
    pub token_mint: Pubkey,
    pub token_treasury_pda: Pubkey,
    pub token_program: Pubkey,
    pub system_program: Pubkey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateIntentParams {
    pub intent_id: u64,
    pub terms_hash: [u8; 32],
    pub deposit_amount: u64,
    pub expiration_ts: i64,
    pub claim_period: u16,
    pub fixture_id: i64,
}

/// Account order:
/// 0. `[writable, signer]` maker
/// 1. `[writable]` order_intent
/// 2. `[writable]` intent_vault
/// 3. `[writable]` maker_token_account
/// 4. `[]` token_mint
/// 5. `[]` token_treasury_pda
/// 6. `[]` token_program
/// 7. `[]` system_program
pub fn create_intent_instruction(
    program_id: Pubkey,
    accounts: CreateIntentAccounts,
    params: CreateIntentParams,
) -> Result<Instruction> {
    let mut data = Vec::new();
    data.extend_from_slice(&CREATE_INTENT_DISCRIMINATOR);
    put_u64(&mut data, params.intent_id);
    data.extend_from_slice(&params.terms_hash);
    put_u64(&mut data, params.deposit_amount);
    put_i64(&mut data, params.expiration_ts);
    put_u16(&mut data, params.claim_period);
    put_i64(&mut data, params.fixture_id);

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.maker, true),
            AccountMeta::new(accounts.order_intent, false),
            AccountMeta::new(accounts.intent_vault, false),
            AccountMeta::new(accounts.maker_token_account, false),
            AccountMeta::new_readonly(accounts.token_mint, false),
            AccountMeta::new_readonly(accounts.token_treasury_pda, false),
            AccountMeta::new_readonly(accounts.token_program, false),
            AccountMeta::new_readonly(accounts.system_program, false),
        ],
        data,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTradeAccounts {
    pub authority: Pubkey,
    pub trader_a: Pubkey,
    pub trader_b: Pubkey,
    pub trader_a_token_account: Pubkey,
    pub trader_b_token_account: Pubkey,
    pub trade_escrow: Pubkey,
    pub escrow_vault: Pubkey,
    pub stake_token_mint: Pubkey,
    pub token_treasury_pda: Pubkey,
    pub token_program: Pubkey,
    pub system_program: Pubkey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTradeParams {
    pub trade_id: u64,
    pub stake_a: u64,
    pub stake_b: u64,
    pub trade_terms_hash: [u8; 32],
}

/// Account order:
/// 0. `[writable, signer]` authority
/// 1. `[writable, signer]` trader_a
/// 2. `[writable, signer]` trader_b
/// 3. `[writable]` trader_a_token_account
/// 4. `[writable]` trader_b_token_account
/// 5. `[writable]` trade_escrow
/// 6. `[writable]` escrow_vault
/// 7. `[]` stake_token_mint
/// 8. `[]` token_treasury_pda
/// 9. `[]` token_program
/// 10. `[]` system_program
pub fn create_trade_instruction(
    program_id: Pubkey,
    accounts: CreateTradeAccounts,
    params: CreateTradeParams,
) -> Result<Instruction> {
    let mut data = Vec::new();
    data.extend_from_slice(&CREATE_TRADE_DISCRIMINATOR);
    put_u64(&mut data, params.trade_id);
    put_u64(&mut data, params.stake_a);
    put_u64(&mut data, params.stake_b);
    data.extend_from_slice(&params.trade_terms_hash);

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.authority, true),
            AccountMeta::new(accounts.trader_a, true),
            AccountMeta::new(accounts.trader_b, true),
            AccountMeta::new(accounts.trader_a_token_account, false),
            AccountMeta::new(accounts.trader_b_token_account, false),
            AccountMeta::new(accounts.trade_escrow, false),
            AccountMeta::new(accounts.escrow_vault, false),
            AccountMeta::new_readonly(accounts.stake_token_mint, false),
            AccountMeta::new_readonly(accounts.token_treasury_pda, false),
            AccountMeta::new_readonly(accounts.token_program, false),
            AccountMeta::new_readonly(accounts.system_program, false),
        ],
        data,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteMatchAccounts {
    pub solver: Pubkey,
    pub maker_intent: Pubkey,
    pub taker_intent: Pubkey,
    pub maker_vault: Pubkey,
    pub taker_vault: Pubkey,
    pub matched_trade: Pubkey,
    pub trade_vault: Pubkey,
    pub token_mint: Pubkey,
    pub token_program: Pubkey,
    pub system_program: Pubkey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteMatchParams {
    pub trade_id: u64,
    pub maker_stake: u64,
    pub taker_stake: u64,
}

/// Account order:
/// 0. `[writable, signer]` solver
/// 1. `[writable]` maker_intent
/// 2. `[writable]` taker_intent
/// 3. `[writable]` maker_vault
/// 4. `[writable]` taker_vault
/// 5. `[writable]` matched_trade
/// 6. `[writable]` trade_vault
/// 7. `[]` token_mint
/// 8. `[]` token_program
/// 9. `[]` system_program
pub fn execute_match_instruction(
    program_id: Pubkey,
    accounts: ExecuteMatchAccounts,
    params: ExecuteMatchParams,
) -> Result<Instruction> {
    let mut data = Vec::new();
    data.extend_from_slice(&EXECUTE_MATCH_DISCRIMINATOR);
    put_u64(&mut data, params.trade_id);
    put_u64(&mut data, params.maker_stake);
    put_u64(&mut data, params.taker_stake);

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.solver, true),
            AccountMeta::new(accounts.maker_intent, false),
            AccountMeta::new(accounts.taker_intent, false),
            AccountMeta::new(accounts.maker_vault, false),
            AccountMeta::new(accounts.taker_vault, false),
            AccountMeta::new(accounts.matched_trade, false),
            AccountMeta::new(accounts.trade_vault, false),
            AccountMeta::new_readonly(accounts.token_mint, false),
            AccountMeta::new_readonly(accounts.token_program, false),
            AccountMeta::new_readonly(accounts.system_program, false),
        ],
        data,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseIntentAccounts {
    pub maker: Pubkey,
    pub authority: Pubkey,
    pub order_intent: Pubkey,
    pub intent_vault: Pubkey,
    pub maker_token_account: Pubkey,
    pub token_mint: Pubkey,
    pub token_program: Pubkey,
    pub token_treasury_pda: Pubkey,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CloseIntentParams {}

/// Account order:
/// 0. `[writable]` maker
/// 1. `[writable, signer]` authority
/// 2. `[writable]` order_intent
/// 3. `[writable]` intent_vault
/// 4. `[writable]` maker_token_account
/// 5. `[]` token_mint
/// 6. `[]` token_program
/// 7. `[]` token_treasury_pda
pub fn close_intent_instruction(
    program_id: Pubkey,
    accounts: CloseIntentAccounts,
    _params: CloseIntentParams,
) -> Result<Instruction> {
    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.maker, false),
            AccountMeta::new(accounts.authority, true),
            AccountMeta::new(accounts.order_intent, false),
            AccountMeta::new(accounts.intent_vault, false),
            AccountMeta::new(accounts.maker_token_account, false),
            AccountMeta::new_readonly(accounts.token_mint, false),
            AccountMeta::new_readonly(accounts.token_program, false),
            AccountMeta::new_readonly(accounts.token_treasury_pda, false),
        ],
        data: CLOSE_INTENT_DISCRIMINATOR.to_vec(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleTradeAccounts {
    pub winner: Pubkey,
    pub daily_scores_merkle_roots: Pubkey,
    pub trade_escrow: Pubkey,
    pub escrow_vault: Pubkey,
    pub winner_token_account: Pubkey,
    pub token_mint: Pubkey,
    pub token_treasury_pda: Pubkey,
    pub token_program: Pubkey,
    pub system_program: Pubkey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleTradeParams {
    pub trade_id: u64,
    pub ts: i64,
    pub fixture_summary: FixtureSummaryInput,
    pub fixture_proof: Vec<ProofNode>,
    pub main_tree_proof: Vec<ProofNode>,
    pub predicate: TraderPredicate,
    pub stat_a: StatTermInput,
    pub stat_b: Option<StatTermInput>,
    pub op: Option<BinaryExpression>,
}

/// Account order:
/// 0. `[writable, signer]` winner
/// 1. `[]` daily_scores_merkle_roots
/// 2. `[writable]` trade_escrow
/// 3. `[writable]` escrow_vault
/// 4. `[writable]` winner_token_account
/// 5. `[]` token_mint
/// 6. `[]` token_treasury_pda
/// 7. `[]` token_program
/// 8. `[]` system_program
pub fn settle_trade_instruction(
    program_id: Pubkey,
    accounts: SettleTradeAccounts,
    params: SettleTradeParams,
) -> Result<Instruction> {
    let mut data = Vec::new();
    data.extend_from_slice(&SETTLE_TRADE_DISCRIMINATOR);
    put_u64(&mut data, params.trade_id);
    put_i64(&mut data, params.ts);
    encode_scores_batch_summary(&mut data, &params.fixture_summary);
    encode_proof_vec(&mut data, &params.fixture_proof)?;
    encode_proof_vec(&mut data, &params.main_tree_proof)?;
    encode_trader_predicate(&mut data, &params.predicate);
    encode_stat_term(&mut data, &params.stat_a)?;
    encode_option(&mut data, params.stat_b.as_ref(), encode_stat_term)?;
    encode_option(&mut data, params.op.as_ref(), |out, op| {
        encode_binary_expression(out, op);
        Ok(())
    })?;

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.winner, true),
            AccountMeta::new_readonly(accounts.daily_scores_merkle_roots, false),
            AccountMeta::new(accounts.trade_escrow, false),
            AccountMeta::new(accounts.escrow_vault, false),
            AccountMeta::new(accounts.winner_token_account, false),
            AccountMeta::new_readonly(accounts.token_mint, false),
            AccountMeta::new_readonly(accounts.token_treasury_pda, false),
            AccountMeta::new_readonly(accounts.token_program, false),
            AccountMeta::new_readonly(accounts.system_program, false),
        ],
        data,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleMatchedTradeAccounts {
    pub winner: Pubkey,
    pub daily_scores_merkle_roots: Pubkey,
    pub matched_trade: Pubkey,
    pub trade_vault: Pubkey,
    pub winner_token_account: Pubkey,
    pub token_mint: Pubkey,
    pub token_treasury_pda: Pubkey,
    pub token_program: Pubkey,
    pub system_program: Pubkey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleMatchedTradeParams {
    pub trade_id: u64,
    pub ts: i64,
    pub fixture_summary: FixtureSummaryInput,
    pub fixture_proof: Vec<ProofNode>,
    pub main_tree_proof: Vec<ProofNode>,
    pub stat_a: StatTermInput,
    pub stat_b: Option<StatTermInput>,
    pub terms: MarketIntentParams,
}

/// Account order:
/// 0. `[writable, signer]` winner
/// 1. `[]` daily_scores_merkle_roots
/// 2. `[writable]` matched_trade
/// 3. `[writable]` trade_vault
/// 4. `[writable]` winner_token_account
/// 5. `[]` token_mint
/// 6. `[]` token_treasury_pda
/// 7. `[]` token_program
/// 8. `[]` system_program
pub fn settle_matched_trade_instruction(
    program_id: Pubkey,
    accounts: SettleMatchedTradeAccounts,
    params: SettleMatchedTradeParams,
) -> Result<Instruction> {
    let mut data = Vec::new();
    data.extend_from_slice(&SETTLE_MATCHED_TRADE_DISCRIMINATOR);
    put_u64(&mut data, params.trade_id);
    put_i64(&mut data, params.ts);
    encode_scores_batch_summary(&mut data, &params.fixture_summary);
    encode_proof_vec(&mut data, &params.fixture_proof)?;
    encode_proof_vec(&mut data, &params.main_tree_proof)?;
    encode_stat_term(&mut data, &params.stat_a)?;
    encode_option(&mut data, params.stat_b.as_ref(), encode_stat_term)?;
    encode_market_intent_params(&mut data, &params.terms)?;

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.winner, true),
            AccountMeta::new_readonly(accounts.daily_scores_merkle_roots, false),
            AccountMeta::new(accounts.matched_trade, false),
            AccountMeta::new(accounts.trade_vault, false),
            AccountMeta::new(accounts.winner_token_account, false),
            AccountMeta::new_readonly(accounts.token_mint, false),
            AccountMeta::new_readonly(accounts.token_treasury_pda, false),
            AccountMeta::new_readonly(accounts.token_program, false),
            AccountMeta::new_readonly(accounts.system_program, false),
        ],
        data,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimViaResolutionAccounts {
    pub winner: Pubkey,
    pub daily_resolution_roots: Pubkey,
    pub matched_trade: Pubkey,
    pub trade_vault: Pubkey,
    pub winner_token_account: Pubkey,
    pub token_program: Pubkey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimViaResolutionParams {
    pub epoch_day: u16,
    pub interval_index: u16,
    pub merkle_proof: Vec<ProofNode>,
}

/// Account order:
/// 0. `[writable, signer]` winner
/// 1. `[]` daily_resolution_roots
/// 2. `[writable]` matched_trade
/// 3. `[writable]` trade_vault
/// 4. `[writable]` winner_token_account
/// 5. `[]` token_program
pub fn claim_via_resolution_instruction(
    program_id: Pubkey,
    accounts: ClaimViaResolutionAccounts,
    params: ClaimViaResolutionParams,
) -> Result<Instruction> {
    let mut data = Vec::new();
    data.extend_from_slice(&CLAIM_VIA_RESOLUTION_DISCRIMINATOR);
    put_u16(&mut data, params.epoch_day);
    put_u16(&mut data, params.interval_index);
    encode_proof_vec(&mut data, &params.merkle_proof)?;

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.winner, true),
            AccountMeta::new_readonly(accounts.daily_resolution_roots, false),
            AccountMeta::new(accounts.matched_trade, false),
            AccountMeta::new(accounts.trade_vault, false),
            AccountMeta::new(accounts.winner_token_account, false),
            AccountMeta::new_readonly(accounts.token_program, false),
        ],
        data,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimBatchLegacyAccounts {
    pub payer: Pubkey,
    pub daily_resolution_roots: Pubkey,
    pub token_mint: Pubkey,
    pub token_program: Pubkey,
    pub system_program: Pubkey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimBatchLegacyParams {
    pub epoch_day: u16,
    pub interval_index: u16,
    pub terms_hash: [u8; 32],
    pub winner_is_maker: bool,
    pub seq: u32,
    pub merkle_proof: Vec<ProofNode>,
}

/// Account order:
/// 0. `[writable, signer]` payer
/// 1. `[]` daily_resolution_roots
/// 2. `[]` token_mint
/// 3. `[]` token_program
/// 4. `[]` system_program
pub fn claim_batch_legacy_instruction(
    program_id: Pubkey,
    accounts: ClaimBatchLegacyAccounts,
    params: ClaimBatchLegacyParams,
) -> Result<Instruction> {
    let mut data = Vec::new();
    data.extend_from_slice(&CLAIM_BATCH_LEGACY_DISCRIMINATOR);
    put_u16(&mut data, params.epoch_day);
    put_u16(&mut data, params.interval_index);
    data.extend_from_slice(&params.terms_hash);
    put_bool(&mut data, params.winner_is_maker);
    put_u32(&mut data, params.seq);
    encode_proof_vec(&mut data, &params.merkle_proof)?;

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.payer, true),
            AccountMeta::new_readonly(accounts.daily_resolution_roots, false),
            AccountMeta::new_readonly(accounts.token_mint, false),
            AccountMeta::new_readonly(accounts.token_program, false),
            AccountMeta::new_readonly(accounts.system_program, false),
        ],
        data,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefundBatchAccounts {
    pub payer: Pubkey,
    pub token_mint: Pubkey,
    pub token_program: Pubkey,
    pub system_program: Pubkey,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefundBatchParams {}

/// Account order:
/// 0. `[writable, signer]` payer
/// 1. `[]` token_mint
/// 2. `[]` token_program
/// 3. `[]` system_program
pub fn refund_batch_instruction(
    program_id: Pubkey,
    accounts: RefundBatchAccounts,
    _params: RefundBatchParams,
) -> Result<Instruction> {
    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.payer, true),
            AccountMeta::new_readonly(accounts.token_mint, false),
            AccountMeta::new_readonly(accounts.token_program, false),
            AccountMeta::new_readonly(accounts.system_program, false),
        ],
        data: REFUND_BATCH_DISCRIMINATOR.to_vec(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditTradeResultAccounts {
    pub payer: Pubkey,
    pub daily_scores_merkle_roots: Pubkey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditTradeResultParams {
    pub terms: MarketIntentParams,
    pub fixture_summary: FixtureSummaryInput,
    pub main_tree_proof: Vec<ProofNode>,
    pub fixture_proof: Vec<ProofNode>,
    pub stat_a: StatTermInput,
    pub stat_b: Option<StatTermInput>,
    pub ts: i64,
}

/// Account order:
/// 0. `[writable, signer]` payer
/// 1. `[]` daily_scores_merkle_roots
pub fn audit_trade_result_instruction(
    program_id: Pubkey,
    accounts: AuditTradeResultAccounts,
    params: AuditTradeResultParams,
) -> Result<Instruction> {
    let mut data = Vec::new();
    data.extend_from_slice(&AUDIT_TRADE_RESULT_DISCRIMINATOR);
    encode_market_intent_params(&mut data, &params.terms)?;
    encode_scores_batch_summary(&mut data, &params.fixture_summary);
    encode_proof_vec(&mut data, &params.main_tree_proof)?;
    encode_proof_vec(&mut data, &params.fixture_proof)?;
    encode_stat_term(&mut data, &params.stat_a)?;
    encode_option(&mut data, params.stat_b.as_ref(), encode_stat_term)?;
    put_i64(&mut data, params.ts);

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.payer, true),
            AccountMeta::new_readonly(accounts.daily_scores_merkle_roots, false),
        ],
        data,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketIntentParams {
    pub fixture_id: i64,
    pub period: u16,
    pub stat_a_key: u32,
    pub stat_b_key: Option<u32>,
    pub predicate: TraderPredicate,
    pub op: Option<BinaryExpression>,
    pub negation: bool,
}

fn encode_market_intent_params(out: &mut Vec<u8>, terms: &MarketIntentParams) -> Result<()> {
    put_i64(out, terms.fixture_id);
    put_u16(out, terms.period);
    put_u32(out, terms.stat_a_key);
    encode_option(out, terms.stat_b_key.as_ref(), |out, value| {
        put_u32(out, *value);
        Ok(())
    })?;
    encode_trader_predicate(out, &terms.predicate);
    encode_option(out, terms.op.as_ref(), |out, op| {
        encode_binary_expression(out, op);
        Ok(())
    })?;
    put_bool(out, terms.negation);
    Ok(())
}
