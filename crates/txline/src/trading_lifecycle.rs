//! Devnet trading lifecycle helpers for score-settled market demos.
//!
//! The helpers in this module compose the published TxLINE Devnet score APIs,
//! validation payloads, and public TxODDS Devnet IDL builders. They do not
//! invent trading REST endpoints, derive unpublished trading PDAs, sign
//! transactions, or compute a production terms hash preimage.
//!
//! ```no_run
//! use txline::{
//!     ApiToken, FinalOutcomeConfig, GuestJwt, TxlineClient, TxlineConfig,
//!     extract_final_outcome, final_outcome_validation_plan, is_final_outcome_record,
//! };
//! use txline::solana::pda::parse_pubkey;
//!
//! # async fn run() -> txline::Result<()> {
//! let client = TxlineClient::new(TxlineConfig::devnet())?;
//! client.set_guest_jwt(GuestJwt::new("guest-jwt")?);
//! client.set_api_token(ApiToken::new("activated-api-token")?);
//!
//! let fixture_id = 17_952_170;
//! let scores = client.scores().historical_by_fixture(fixture_id).await?;
//! let Some(final_score) = scores.iter().find(|score| is_final_outcome_record(score)) else {
//!     return Ok(());
//! };
//! let outcome = extract_final_outcome(final_score, FinalOutcomeConfig::soccer_default())?;
//!
//! let validation = client
//!     .scores()
//!     .stat_validation_v2(outcome.fixture_id, outcome.seq, outcome.stat_keys())
//!     .await?;
//!
//! let program_id = parse_pubkey(client.config().program_id.as_str())?;
//! let plan = final_outcome_validation_plan(program_id, &validation, &outcome)?;
//! let ix = plan.instructions[0].clone();
//! # let _ = ix;
//! # Ok(())
//! # }
//! ```

use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

use crate::http::models::Scores;
use crate::solana::trading::{
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
use crate::solana::validation::devnet_validate_stat_v2_instruction;
use crate::validation::legacy::ScoresStatValidation;
use crate::validation::strategy::{
    BinaryExpression, Comparison, NDimensionalStrategy, TraderPredicate,
};
use crate::validation::v2::{ScoresStatValidationV2, StatValidationInput};
use crate::{Result, TxlineError};

/// Explicit 32-byte terms hash supplied by the coordinating application.
///
/// The current public Devnet docs and IDL define `[u8; 32]` hash arguments for
/// `create_intent`, `create_trade`, and legacy batch claims, but do not define a
/// canonical production preimage. This wrapper keeps the SDK lifecycle API
/// explicit about that boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermsHash([u8; 32]);

impl TermsHash {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl From<[u8; 32]> for TermsHash {
    fn from(value: [u8; 32]) -> Self {
        Self::new(value)
    }
}

/// Market side used by score-based outcome helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarketSide {
    Participant1,
    Participant2,
    Draw,
}

/// Supported score-market categories for lifecycle planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScoreMarketKind {
    FinalOutcome,
    TotalGoals,
    Spread,
}

/// Score-market terms that map directly to Devnet `MarketIntentParams`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreMarketTerms {
    pub fixture_id: i64,
    pub kind: ScoreMarketKind,
    pub period: u16,
    pub stat_a_key: u32,
    pub stat_b_key: Option<u32>,
    pub predicate: TraderPredicate,
    pub op: Option<BinaryExpression>,
    pub negation: bool,
}

impl ScoreMarketTerms {
    /// Build final-outcome terms using the documented full-game score stats.
    pub fn final_outcome(fixture_id: i64, side: MarketSide, cfg: FinalOutcomeConfig) -> Self {
        let (stat_a_key, stat_b_key, comparison) = match side {
            MarketSide::Participant1 => (
                cfg.participant1_goals_stat_key,
                cfg.participant2_goals_stat_key,
                Comparison::greater_than(),
            ),
            MarketSide::Participant2 => (
                cfg.participant2_goals_stat_key,
                cfg.participant1_goals_stat_key,
                Comparison::greater_than(),
            ),
            MarketSide::Draw => (
                cfg.participant1_goals_stat_key,
                cfg.participant2_goals_stat_key,
                Comparison::equal_to(),
            ),
        };
        Self {
            fixture_id,
            kind: ScoreMarketKind::FinalOutcome,
            period: cfg.period,
            stat_a_key,
            stat_b_key: Some(stat_b_key),
            predicate: TraderPredicate::new(0, comparison),
            op: Some(BinaryExpression::subtract()),
            negation: false,
        }
    }

    /// Build total-goals terms as `participant1_goals + participant2_goals`.
    pub fn total_goals(
        fixture_id: i64,
        threshold: i32,
        comparison: Comparison,
        cfg: FinalOutcomeConfig,
    ) -> Self {
        Self {
            fixture_id,
            kind: ScoreMarketKind::TotalGoals,
            period: cfg.period,
            stat_a_key: cfg.participant1_goals_stat_key,
            stat_b_key: Some(cfg.participant2_goals_stat_key),
            predicate: TraderPredicate::new(threshold, comparison),
            op: Some(BinaryExpression::add()),
            negation: false,
        }
    }

    /// Build spread terms as `stat_a - stat_b`.
    pub fn spread(
        fixture_id: i64,
        period: u16,
        stat_a_key: u32,
        stat_b_key: u32,
        predicate: TraderPredicate,
    ) -> Self {
        Self {
            fixture_id,
            kind: ScoreMarketKind::Spread,
            period,
            stat_a_key,
            stat_b_key: Some(stat_b_key),
            predicate,
            op: Some(BinaryExpression::subtract()),
            negation: false,
        }
    }

    pub fn stat_keys(&self) -> Vec<u32> {
        let mut keys = vec![self.stat_a_key];
        if let Some(stat_b_key) = self.stat_b_key {
            keys.push(stat_b_key);
        }
        keys
    }

    pub fn to_market_intent_params(&self) -> MarketIntentParams {
        MarketIntentParams {
            fixture_id: self.fixture_id,
            period: self.period,
            stat_a_key: self.stat_a_key,
            stat_b_key: self.stat_b_key,
            predicate: self.predicate,
            op: self.op,
            negation: self.negation,
        }
    }

    /// Build a V2 validation strategy using stat indexes in `stat_keys()` order.
    pub fn strategy(&self) -> Result<NDimensionalStrategy> {
        if self.negation {
            return Err(TxlineError::validation(
                "V2 validation strategy helpers do not encode negated market terms",
            ));
        }
        match (self.stat_b_key, self.op) {
            (Some(_), Some(op)) => NDimensionalStrategy::builder(2)
                .binary(0, 1, op, self.predicate)?
                .build(),
            (None, None) => NDimensionalStrategy::builder(1)
                .single(0, self.predicate)?
                .build(),
            _ => Err(TxlineError::validation(
                "score market strategy requires stat_b_key and op to be both present or both absent",
            )),
        }
    }
}

/// Configuration for extracting a documented final score record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalOutcomeConfig {
    pub period: u16,
    pub participant1_goals_stat_key: u32,
    pub participant2_goals_stat_key: u32,
}

impl FinalOutcomeConfig {
    pub const fn soccer_default() -> Self {
        Self {
            period: 100,
            participant1_goals_stat_key: 1,
            participant2_goals_stat_key: 2,
        }
    }

    pub const fn stat_keys(&self) -> [u32; 2] {
        [
            self.participant1_goals_stat_key,
            self.participant2_goals_stat_key,
        ]
    }
}

/// Extracted final outcome and the score stat keys used to prove it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalOutcome {
    pub fixture_id: i64,
    pub seq: i32,
    pub period: u16,
    pub participant1_score: i32,
    pub participant2_score: i32,
    pub participant1_goals_stat_key: u32,
    pub participant2_goals_stat_key: u32,
    pub result: MarketSide,
}

impl FinalOutcome {
    pub const fn stat_keys(&self) -> [u32; 2] {
        [
            self.participant1_goals_stat_key,
            self.participant2_goals_stat_key,
        ]
    }

    pub fn market_terms(&self) -> ScoreMarketTerms {
        ScoreMarketTerms::final_outcome(
            self.fixture_id,
            self.result,
            FinalOutcomeConfig {
                period: self.period,
                participant1_goals_stat_key: self.participant1_goals_stat_key,
                participant2_goals_stat_key: self.participant2_goals_stat_key,
            },
        )
    }
}

pub fn is_final_outcome_record(score: &Scores) -> bool {
    score.is_final_outcome_record()
}

pub fn extract_final_outcome(score: &Scores, cfg: FinalOutcomeConfig) -> Result<FinalOutcome> {
    if !is_final_outcome_record(score) {
        return Err(TxlineError::validation(
            "score record is not a final outcome record; expected action=game_finalised, statusId=100, period=100",
        ));
    }
    if score.seq <= 0 {
        return Err(TxlineError::validation(
            "final outcome score seq must be greater than zero",
        ));
    }
    let participant1_score = stat_value(score, cfg.participant1_goals_stat_key)?;
    let participant2_score = stat_value(score, cfg.participant2_goals_stat_key)?;
    let result = match participant1_score.cmp(&participant2_score) {
        std::cmp::Ordering::Greater => MarketSide::Participant1,
        std::cmp::Ordering::Less => MarketSide::Participant2,
        std::cmp::Ordering::Equal => MarketSide::Draw,
    };

    Ok(FinalOutcome {
        fixture_id: score.fixture_id,
        seq: score.seq,
        period: cfg.period,
        participant1_score,
        participant2_score,
        participant1_goals_stat_key: cfg.participant1_goals_stat_key,
        participant2_goals_stat_key: cfg.participant2_goals_stat_key,
        result,
    })
}

pub fn final_outcome_side_strategy(side: MarketSide) -> Result<NDimensionalStrategy> {
    match side {
        MarketSide::Participant1 => NDimensionalStrategy::builder(2)
            .binary(
                0,
                1,
                BinaryExpression::subtract(),
                TraderPredicate::new(0, Comparison::greater_than()),
            )?
            .build(),
        MarketSide::Participant2 => NDimensionalStrategy::builder(2)
            .binary(
                1,
                0,
                BinaryExpression::subtract(),
                TraderPredicate::new(0, Comparison::greater_than()),
            )?
            .build(),
        MarketSide::Draw => NDimensionalStrategy::builder(2)
            .binary(
                0,
                1,
                BinaryExpression::subtract(),
                TraderPredicate::new(0, Comparison::equal_to()),
            )?
            .build(),
    }
}

pub fn final_outcome_strategy(outcome: &FinalOutcome) -> Result<NDimensionalStrategy> {
    final_outcome_side_strategy(outcome.result)
}

pub fn validation_input_for_market(
    validation: &ScoresStatValidationV2,
    terms: &ScoreMarketTerms,
) -> Result<StatValidationInput> {
    let expected = terms.stat_keys();
    if validation.requested_stat_keys() != expected.as_slice() {
        return Err(TxlineError::validation(format!(
            "validation stat key order {:?} does not match market stat key order {:?}",
            validation.requested_stat_keys(),
            expected
        )));
    }
    let input = validation.to_validation_input();
    if input.fixture_summary.fixture_id != terms.fixture_id {
        return Err(TxlineError::validation(format!(
            "validation fixture_id {} does not match market fixture_id {}",
            input.fixture_summary.fixture_id, terms.fixture_id
        )));
    }
    Ok(input)
}

pub fn final_outcome_validation_plan(
    program_id: Pubkey,
    validation: &ScoresStatValidationV2,
    outcome: &FinalOutcome,
) -> Result<LifecyclePlan> {
    let terms = outcome.market_terms();
    let payload = validation_input_for_market(validation, &terms)?;
    let strategy = final_outcome_strategy(outcome)?;
    let instruction = devnet_validate_stat_v2_instruction(program_id, &payload, &strategy)?;
    Ok(LifecyclePlan::single(
        LifecycleAction::ValidateFinalOutcome,
        instruction,
        &[
            "simulate validate_stat_v2 against Devnet before using the result for settlement",
            "use caller-owned market accounts to settle, claim, refund, or audit as appropriate",
        ],
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleAction {
    CreateIntent,
    CloseIntent,
    CreateTrade,
    ExecuteMatch,
    ValidateFinalOutcome,
    SettleTrade,
    SettleMatchedTrade,
    ClaimViaResolution,
    ClaimBatchLegacy,
    RefundBatch,
    AuditTradeResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecyclePlan {
    pub action: LifecycleAction,
    pub instructions: Vec<Instruction>,
    pub next_steps: Vec<&'static str>,
}

impl LifecyclePlan {
    pub fn single(
        action: LifecycleAction,
        instruction: Instruction,
        next_steps: &[&'static str],
    ) -> Self {
        Self {
            action,
            instructions: vec![instruction],
            next_steps: next_steps.to_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateIntentPlanParams {
    pub intent_id: u64,
    pub terms_hash: TermsHash,
    pub deposit_amount: u64,
    pub expiration_ts: i64,
    pub claim_period: u16,
    pub market: ScoreMarketTerms,
}

impl CreateIntentPlanParams {
    pub fn to_instruction_params(&self) -> CreateIntentParams {
        CreateIntentParams {
            intent_id: self.intent_id,
            terms_hash: self.terms_hash.into_bytes(),
            deposit_amount: self.deposit_amount,
            expiration_ts: self.expiration_ts,
            claim_period: self.claim_period,
            fixture_id: self.market.fixture_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTradePlanParams {
    pub trade_id: u64,
    pub stake_a: u64,
    pub stake_b: u64,
    pub terms_hash: TermsHash,
}

impl CreateTradePlanParams {
    pub fn to_instruction_params(&self) -> CreateTradeParams {
        CreateTradeParams {
            trade_id: self.trade_id,
            stake_a: self.stake_a,
            stake_b: self.stake_b,
            trade_terms_hash: self.terms_hash.into_bytes(),
        }
    }
}

pub fn create_intent_plan(
    program_id: Pubkey,
    accounts: CreateIntentAccounts,
    params: CreateIntentPlanParams,
) -> Result<LifecyclePlan> {
    let instruction =
        create_intent_instruction(program_id, accounts, params.to_instruction_params())?;
    Ok(LifecyclePlan::single(
        LifecycleAction::CreateIntent,
        instruction,
        &[
            "caller supplies and persists the order_intent and intent_vault accounts",
            "sign and submit with the maker after reviewing the explicit terms hash",
        ],
    ))
}

pub fn close_intent_plan(
    program_id: Pubkey,
    accounts: CloseIntentAccounts,
    params: CloseIntentParams,
) -> Result<LifecyclePlan> {
    let instruction = close_intent_instruction(program_id, accounts, params)?;
    Ok(LifecyclePlan::single(
        LifecycleAction::CloseIntent,
        instruction,
        &["sign and submit with the intent authority"],
    ))
}

pub fn create_trade_plan(
    program_id: Pubkey,
    accounts: CreateTradeAccounts,
    params: CreateTradePlanParams,
) -> Result<LifecyclePlan> {
    let instruction =
        create_trade_instruction(program_id, accounts, params.to_instruction_params())?;
    Ok(LifecyclePlan::single(
        LifecycleAction::CreateTrade,
        instruction,
        &[
            "caller supplies all escrow accounts and both trader signatures",
            "review the explicit trade terms hash before signing",
        ],
    ))
}

pub fn execute_match_plan(
    program_id: Pubkey,
    accounts: ExecuteMatchAccounts,
    params: ExecuteMatchParams,
) -> Result<LifecyclePlan> {
    let instruction = execute_match_instruction(program_id, accounts, params)?;
    Ok(LifecyclePlan::single(
        LifecycleAction::ExecuteMatch,
        instruction,
        &["caller supplies matched trade accounts and signs with the solver"],
    ))
}

pub fn settle_trade_plan(
    program_id: Pubkey,
    accounts: SettleTradeAccounts,
    params: SettleTradeParams,
) -> Result<LifecyclePlan> {
    let instruction = settle_trade_instruction(program_id, accounts, params)?;
    Ok(LifecyclePlan::single(
        LifecycleAction::SettleTrade,
        instruction,
        &["sign and submit with the winning trader after proof review or simulation"],
    ))
}

pub fn settle_matched_trade_plan(
    program_id: Pubkey,
    accounts: SettleMatchedTradeAccounts,
    params: SettleMatchedTradeParams,
) -> Result<LifecyclePlan> {
    let instruction = settle_matched_trade_instruction(program_id, accounts, params)?;
    Ok(LifecyclePlan::single(
        LifecycleAction::SettleMatchedTrade,
        instruction,
        &["sign and submit with the winning trader after proof review or simulation"],
    ))
}

pub fn claim_via_resolution_plan(
    program_id: Pubkey,
    accounts: ClaimViaResolutionAccounts,
    params: ClaimViaResolutionParams,
) -> Result<LifecyclePlan> {
    let instruction = claim_via_resolution_instruction(program_id, accounts, params)?;
    Ok(LifecyclePlan::single(
        LifecycleAction::ClaimViaResolution,
        instruction,
        &["caller supplies the published resolution root and proof"],
    ))
}

pub fn claim_batch_legacy_plan(
    program_id: Pubkey,
    accounts: ClaimBatchLegacyAccounts,
    params: ClaimBatchLegacyParams,
) -> Result<LifecyclePlan> {
    let instruction = claim_batch_legacy_instruction(program_id, accounts, params)?;
    Ok(LifecyclePlan::single(
        LifecycleAction::ClaimBatchLegacy,
        instruction,
        &["caller supplies the legacy batch resolution root and proof"],
    ))
}

pub fn refund_batch_plan(
    program_id: Pubkey,
    accounts: RefundBatchAccounts,
    params: RefundBatchParams,
) -> Result<LifecyclePlan> {
    let instruction = refund_batch_instruction(program_id, accounts, params)?;
    Ok(LifecyclePlan::single(
        LifecycleAction::RefundBatch,
        instruction,
        &["sign and submit with the refund payer after confirming eligibility"],
    ))
}

pub fn audit_trade_result_plan(
    program_id: Pubkey,
    accounts: AuditTradeResultAccounts,
    params: AuditTradeResultParams,
) -> Result<LifecyclePlan> {
    let instruction = audit_trade_result_instruction(program_id, accounts, params)?;
    Ok(LifecyclePlan::single(
        LifecycleAction::AuditTradeResult,
        instruction,
        &["simulate or submit with caller-owned audit payer"],
    ))
}

pub fn settle_trade_params_from_legacy_validation(
    trade_id: u64,
    validation: &ScoresStatValidation,
    terms: &ScoreMarketTerms,
) -> Result<SettleTradeParams> {
    if terms.negation {
        return Err(TxlineError::validation(
            "direct trade settlement params do not carry market negation",
        ));
    }
    ensure_legacy_validation_matches_terms(validation, terms)?;
    Ok(SettleTradeParams {
        trade_id,
        ts: validation.summary.update_stats.min_timestamp,
        fixture_summary: validation.fixture_summary_input(),
        fixture_proof: validation.sub_tree_proof.clone(),
        main_tree_proof: validation.main_tree_proof.clone(),
        predicate: terms.predicate,
        stat_a: validation.primary_stat_term(),
        stat_b: validation.secondary_stat_term()?,
        op: terms.op,
    })
}

pub fn settle_matched_trade_params_from_legacy_validation(
    trade_id: u64,
    validation: &ScoresStatValidation,
    terms: &ScoreMarketTerms,
) -> Result<SettleMatchedTradeParams> {
    ensure_legacy_validation_matches_terms(validation, terms)?;
    Ok(SettleMatchedTradeParams {
        trade_id,
        ts: validation.summary.update_stats.min_timestamp,
        fixture_summary: validation.fixture_summary_input(),
        fixture_proof: validation.sub_tree_proof.clone(),
        main_tree_proof: validation.main_tree_proof.clone(),
        stat_a: validation.primary_stat_term(),
        stat_b: validation.secondary_stat_term()?,
        terms: terms.to_market_intent_params(),
    })
}

pub fn audit_trade_result_params_from_legacy_validation(
    validation: &ScoresStatValidation,
    terms: &ScoreMarketTerms,
) -> Result<AuditTradeResultParams> {
    ensure_legacy_validation_matches_terms(validation, terms)?;
    Ok(AuditTradeResultParams {
        terms: terms.to_market_intent_params(),
        fixture_summary: validation.fixture_summary_input(),
        main_tree_proof: validation.main_tree_proof.clone(),
        fixture_proof: validation.sub_tree_proof.clone(),
        stat_a: validation.primary_stat_term(),
        stat_b: validation.secondary_stat_term()?,
        ts: validation.summary.update_stats.min_timestamp,
    })
}

fn stat_value(score: &Scores, stat_key: u32) -> Result<i32> {
    let stats = score
        .stats
        .as_ref()
        .ok_or_else(|| TxlineError::validation("final outcome score record has no stats"))?;
    let key = stat_key.to_string();
    stats.get(&key).copied().ok_or_else(|| {
        TxlineError::validation(format!(
            "final outcome score record is missing stat key {stat_key}"
        ))
    })
}

fn ensure_legacy_validation_matches_terms(
    validation: &ScoresStatValidation,
    terms: &ScoreMarketTerms,
) -> Result<()> {
    if validation.summary.fixture_id != terms.fixture_id {
        return Err(TxlineError::validation(format!(
            "validation fixture_id {} does not match market fixture_id {}",
            validation.summary.fixture_id, terms.fixture_id
        )));
    }
    if validation.stat_to_prove.key != terms.stat_a_key {
        return Err(TxlineError::validation(format!(
            "primary validation stat key {} does not match market stat_a_key {}",
            validation.stat_to_prove.key, terms.stat_a_key
        )));
    }
    match (terms.stat_b_key, validation.stat_to_prove2.as_ref()) {
        (Some(expected), Some(actual)) if actual.key == expected => Ok(()),
        (Some(expected), Some(actual)) => Err(TxlineError::validation(format!(
            "secondary validation stat key {} does not match market stat_b_key {}",
            actual.key, expected
        ))),
        (Some(expected), None) => Err(TxlineError::validation(format!(
            "market stat_b_key {expected} requires a secondary validation stat"
        ))),
        (None, Some(_)) => Err(TxlineError::validation(
            "validation includes a secondary stat but market terms do not",
        )),
        (None, None) => Ok(()),
    }
}
