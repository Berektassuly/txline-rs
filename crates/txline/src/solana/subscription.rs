//! Devnet subscription transaction helpers.

use solana_client::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Signature, Signer};
use solana_sdk::transaction::Transaction;

use super::pda::{
    ASSOCIATED_TOKEN_PROGRAM_ID, DevnetPdas, SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, parse_pubkey,
};
use crate::config::TxlineConfig;
use crate::{Result, TxlineError};

pub const SUBSCRIBE_DISCRIMINATOR: [u8; 8] = [254, 28, 191, 138, 156, 179, 183, 53];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscribeAccounts {
    pub user: Pubkey,
    pub pricing_matrix: Pubkey,
    pub token_mint: Pubkey,
    pub user_token_account: Pubkey,
    pub token_treasury_vault: Pubkey,
    pub token_treasury_pda: Pubkey,
    pub token_program: Pubkey,
    pub system_program: Pubkey,
    pub associated_token_program: Pubkey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscribeParams {
    pub service_level_id: u16,
    pub weeks: u8,
}

pub fn validate_subscription_weeks(weeks: u8) -> Result<()> {
    if weeks < 4 || !weeks.is_multiple_of(4) {
        return Err(TxlineError::invalid_input(
            "subscription duration must be at least 4 weeks and a multiple of 4",
        ));
    }
    Ok(())
}

pub fn devnet_subscribe_accounts(user: Pubkey) -> Result<SubscribeAccounts> {
    let pdas = DevnetPdas::new()?;
    Ok(SubscribeAccounts {
        user,
        pricing_matrix: pdas.pricing_matrix().address,
        token_mint: pdas.txl_mint,
        user_token_account: pdas.user_txl_ata(&user)?.address,
        token_treasury_vault: pdas.token_treasury_vault_ata()?.address,
        token_treasury_pda: pdas.token_treasury_v2().address,
        token_program: parse_pubkey(TOKEN_2022_PROGRAM_ID)?,
        system_program: parse_pubkey(SYSTEM_PROGRAM_ID)?,
        associated_token_program: parse_pubkey(ASSOCIATED_TOKEN_PROGRAM_ID)?,
    })
}

pub fn subscribe_instruction(
    program_id: Pubkey,
    accounts: SubscribeAccounts,
    params: SubscribeParams,
) -> Result<Instruction> {
    validate_subscription_weeks(params.weeks)?;
    let mut data = Vec::with_capacity(11);
    data.extend_from_slice(&SUBSCRIBE_DISCRIMINATOR);
    data.extend_from_slice(&params.service_level_id.to_le_bytes());
    data.push(params.weeks);

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.user, true),
            AccountMeta::new_readonly(accounts.pricing_matrix, false),
            AccountMeta::new_readonly(accounts.token_mint, false),
            AccountMeta::new(accounts.user_token_account, false),
            AccountMeta::new(accounts.token_treasury_vault, false),
            AccountMeta::new_readonly(accounts.token_treasury_pda, false),
            AccountMeta::new_readonly(accounts.token_program, false),
            AccountMeta::new_readonly(accounts.system_program, false),
            AccountMeta::new_readonly(accounts.associated_token_program, false),
        ],
        data,
    })
}

pub fn build_subscribe_transaction(
    program_id: Pubkey,
    user: Pubkey,
    service_level_id: u16,
    weeks: u8,
    recent_blockhash: Hash,
) -> Result<Transaction> {
    let accounts = devnet_subscribe_accounts(user)?;
    let instruction = subscribe_instruction(
        program_id,
        accounts,
        SubscribeParams {
            service_level_id,
            weeks,
        },
    )?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&user));
    transaction.message.recent_blockhash = recent_blockhash;
    Ok(transaction)
}

pub fn sign_subscribe_transaction<S: Signer>(
    config: &TxlineConfig,
    signer: &S,
    service_level_id: u16,
    weeks: u8,
    recent_blockhash: Hash,
) -> Result<Transaction> {
    let program_id = parse_pubkey(&config.program_id)?;
    let mut transaction = build_subscribe_transaction(
        program_id,
        signer.pubkey(),
        service_level_id,
        weeks,
        recent_blockhash,
    )?;
    transaction.sign(&[signer], recent_blockhash);
    Ok(transaction)
}

pub fn send_subscribe_transaction<S: Signer>(
    config: &TxlineConfig,
    signer: &S,
    service_level_id: u16,
    weeks: u8,
) -> Result<Signature> {
    let rpc = RpcClient::new(config.rpc_url.clone());
    let blockhash = rpc.get_latest_blockhash()?;
    let transaction =
        sign_subscribe_transaction(config, signer, service_level_id, weeks, blockhash)?;
    Ok(rpc.send_and_confirm_transaction(&transaction)?)
}
