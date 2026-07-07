//! Devnet-only Rust SDK for TxLINE.
//!
//! This crate intentionally supports TxLINE Devnet only in this implementation
//! phase. Mainnet constants, examples, feature flags, and transaction flows are
//! out of scope until the Devnet path has been exercised end to end.

pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod http;
pub mod solana;
pub mod stream;
pub mod trading_lifecycle;
pub mod validation;

pub use auth::{ApiToken, AuthHeaders, GuestJwt, GuestSession, activation_preimage};
pub use client::TxlineClient;
pub use config::{DEVNET_API_BASE, DEVNET_API_HOST, DEVNET_GUEST_AUTH_URL, Network, TxlineConfig};
pub use error::{Result, TxlineError};
pub use solana::transaction_safety::ValidatedPurchaseQuote;
pub use trading_lifecycle::{
    CreateIntentPlanParams, CreateTradePlanParams, FinalOutcome, FinalOutcomeConfig,
    LifecycleAction, LifecyclePlan, MarketSide, ScoreMarketKind, ScoreMarketTerms, TermsHash,
    audit_trade_result_params_from_legacy_validation, audit_trade_result_plan,
    claim_batch_legacy_plan, claim_via_resolution_plan, close_intent_plan, create_intent_plan,
    create_trade_plan, execute_match_plan, extract_final_outcome, final_outcome_side_strategy,
    final_outcome_strategy, final_outcome_validation_plan, is_final_outcome_record,
    refund_batch_plan, settle_matched_trade_params_from_legacy_validation,
    settle_matched_trade_plan, settle_trade_params_from_legacy_validation, settle_trade_plan,
    validation_input_for_market,
};
