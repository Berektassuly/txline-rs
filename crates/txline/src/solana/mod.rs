//! Devnet Solana helpers.

pub mod faucet;
pub mod idl;
pub mod pda;
pub mod purchase;
pub mod setup;
pub mod subscription;
pub mod transaction_safety;
pub mod validation;

use solana_sdk::hash::Hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Signature, Signer};
use solana_sdk::transaction::Transaction;

use crate::Result;
use crate::config::TxlineConfig;
use crate::http::models::{FixtureBatchValidation, FixtureValidation, OddsValidation};
use crate::solana::faucet::{devnet_request_faucet_accounts, request_devnet_faucet_instruction};
use crate::solana::pda::{DevnetPdas, parse_pubkey};
use crate::solana::purchase::{
    devnet_purchase_subscription_token_usdt_accounts, purchase_subscription_token_usdt_instruction,
};
use crate::solana::subscription::{
    build_subscribe_transaction, send_subscribe_transaction, sign_subscribe_transaction,
};
use crate::solana::validation::{
    ValidationSimulationConfig, devnet_validate_fixture_batch_instruction,
    devnet_validate_fixture_instruction, devnet_validate_odds_instruction,
    devnet_validate_stat_instruction, devnet_validate_stat_v2_instruction,
    simulate_validation_instruction,
};
use crate::validation::legacy::ScoresStatValidation;
use crate::validation::strategy::{BinaryExpression, NDimensionalStrategy, TraderPredicate};
use crate::validation::v2::StatValidationInput;

#[derive(Debug, Clone, Copy)]
pub struct SolanaClient<'a> {
    config: &'a TxlineConfig,
}

impl<'a> SolanaClient<'a> {
    pub(crate) fn new(config: &'a TxlineConfig) -> Self {
        Self { config }
    }

    pub fn program_id(&self) -> Result<Pubkey> {
        parse_pubkey(&self.config.program_id)
    }

    pub fn pdas(&self) -> Result<DevnetPdas> {
        DevnetPdas::new()
    }

    pub fn build_subscribe_transaction(
        &self,
        user: Pubkey,
        service_level_id: u16,
        weeks: u8,
        recent_blockhash: Hash,
    ) -> Result<Transaction> {
        build_subscribe_transaction(
            self.program_id()?,
            user,
            service_level_id,
            weeks,
            recent_blockhash,
        )
    }

    pub fn sign_subscribe_transaction<S: Signer>(
        &self,
        signer: &S,
        service_level_id: u16,
        weeks: u8,
        recent_blockhash: Hash,
    ) -> Result<Transaction> {
        sign_subscribe_transaction(
            self.config,
            signer,
            service_level_id,
            weeks,
            recent_blockhash,
        )
    }

    pub fn send_subscribe_transaction<S: Signer>(
        &self,
        signer: &S,
        service_level_id: u16,
        weeks: u8,
    ) -> Result<Signature> {
        send_subscribe_transaction(self.config, signer, service_level_id, weeks)
    }

    pub fn build_purchase_subscription_token_usdt_instruction(
        &self,
        buyer: Pubkey,
        backend_admin: Pubkey,
        txline_amount: u64,
    ) -> Result<Instruction> {
        let accounts = devnet_purchase_subscription_token_usdt_accounts(buyer, backend_admin)?;
        purchase_subscription_token_usdt_instruction(self.program_id()?, accounts, txline_amount)
    }

    pub fn build_request_devnet_faucet_instruction(
        &self,
        user: Pubkey,
        faucet_tracker: Pubkey,
    ) -> Result<Instruction> {
        let accounts = devnet_request_faucet_accounts(user, faucet_tracker)?;
        Ok(request_devnet_faucet_instruction(
            self.program_id()?,
            accounts,
        ))
    }

    pub fn build_validate_stat_instruction(
        &self,
        validation: &ScoresStatValidation,
        predicate: TraderPredicate,
        op: Option<BinaryExpression>,
    ) -> Result<Instruction> {
        devnet_validate_stat_instruction(self.program_id()?, validation, predicate, op)
    }

    pub fn build_validate_stat_v2_instruction(
        &self,
        payload: &StatValidationInput,
        strategy: &NDimensionalStrategy,
    ) -> Result<Instruction> {
        devnet_validate_stat_v2_instruction(self.program_id()?, payload, strategy)
    }

    pub fn build_validate_fixture_instruction(
        &self,
        validation: &FixtureValidation,
    ) -> Result<Instruction> {
        devnet_validate_fixture_instruction(self.program_id()?, validation)
    }

    pub fn build_validate_fixture_batch_instruction(
        &self,
        epoch_day: u16,
        index: u8,
        validation: &FixtureBatchValidation,
    ) -> Result<Instruction> {
        devnet_validate_fixture_batch_instruction(self.program_id()?, epoch_day, index, validation)
    }

    pub fn build_validate_odds_instruction(
        &self,
        validation: &OddsValidation,
    ) -> Result<Instruction> {
        devnet_validate_odds_instruction(self.program_id()?, validation)
    }

    pub fn simulate_validate_stat<S: Signer>(
        &self,
        payer: &S,
        validation: &ScoresStatValidation,
        predicate: TraderPredicate,
        op: Option<BinaryExpression>,
        simulation_config: ValidationSimulationConfig,
    ) -> Result<bool> {
        let instruction = self.build_validate_stat_instruction(validation, predicate, op)?;
        simulate_validation_instruction(self.config, payer, instruction, simulation_config)
    }

    pub fn simulate_validate_stat_v2<S: Signer>(
        &self,
        payer: &S,
        payload: &StatValidationInput,
        strategy: &NDimensionalStrategy,
        simulation_config: ValidationSimulationConfig,
    ) -> Result<bool> {
        let instruction = self.build_validate_stat_v2_instruction(payload, strategy)?;
        simulate_validation_instruction(self.config, payer, instruction, simulation_config)
    }

    pub fn simulate_validate_fixture<S: Signer>(
        &self,
        payer: &S,
        validation: &FixtureValidation,
        simulation_config: ValidationSimulationConfig,
    ) -> Result<bool> {
        let instruction = self.build_validate_fixture_instruction(validation)?;
        simulate_validation_instruction(self.config, payer, instruction, simulation_config)
    }

    pub fn simulate_validate_fixture_batch<S: Signer>(
        &self,
        payer: &S,
        epoch_day: u16,
        index: u8,
        validation: &FixtureBatchValidation,
        simulation_config: ValidationSimulationConfig,
    ) -> Result<bool> {
        let instruction =
            self.build_validate_fixture_batch_instruction(epoch_day, index, validation)?;
        simulate_validation_instruction(self.config, payer, instruction, simulation_config)
    }

    pub fn simulate_validate_odds<S: Signer>(
        &self,
        payer: &S,
        validation: &OddsValidation,
        simulation_config: ValidationSimulationConfig,
    ) -> Result<bool> {
        let instruction = self.build_validate_odds_instruction(validation)?;
        simulate_validation_instruction(self.config, payer, instruction, simulation_config)
    }
}
