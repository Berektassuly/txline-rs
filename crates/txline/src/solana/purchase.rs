//! Purchase quote helpers for paid Devnet flows.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;

use crate::http::models::{PurchaseQuoteRequest, PurchaseQuoteResponse};
use crate::solana::pda::{
    ASSOCIATED_TOKEN_PROGRAM_ID, DevnetPdas, LEGACY_TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID, parse_pubkey,
};
use crate::{Result, TxlineClient, TxlineError};

pub const MAX_QUOTE_TXLINE_AMOUNT: u64 = 100_000_000;
pub const PURCHASE_SUBSCRIPTION_TOKEN_USDT_DISCRIMINATOR: [u8; 8] =
    [198, 251, 223, 9, 31, 184, 166, 188];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseSubscriptionTokenUsdtAccounts {
    pub buyer: Pubkey,
    pub backend_admin: Pubkey,
    pub usdt_mint: Pubkey,
    pub buyer_usdt_account: Pubkey,
    pub usdt_treasury_vault: Pubkey,
    pub usdt_treasury_pda: Pubkey,
    pub subscription_token_mint: Pubkey,
    pub token_treasury_vault: Pubkey,
    pub token_treasury_pda: Pubkey,
    pub buyer_token_account: Pubkey,
    pub token_program: Pubkey,
    pub token_2022_program: Pubkey,
    pub system_program: Pubkey,
    pub associated_token_program: Pubkey,
}

pub async fn purchase_quote(
    client: &TxlineClient,
    buyer_pubkey: impl Into<String>,
    txline_amount: u64,
) -> Result<PurchaseQuoteResponse> {
    validate_quote_amount(txline_amount)?;
    let request = PurchaseQuoteRequest {
        buyer_pubkey: buyer_pubkey.into(),
        txline_amount,
    };
    client
        .post_json("/guest/purchase/quote", &request, false)
        .await
}

pub fn validate_quote_amount(txline_amount: u64) -> Result<()> {
    if txline_amount == 0 || txline_amount > MAX_QUOTE_TXLINE_AMOUNT {
        return Err(TxlineError::invalid_input(format!(
            "txline_amount must be 1..={MAX_QUOTE_TXLINE_AMOUNT}"
        )));
    }
    Ok(())
}

pub fn devnet_purchase_subscription_token_usdt_accounts(
    buyer: Pubkey,
    backend_admin: Pubkey,
) -> Result<PurchaseSubscriptionTokenUsdtAccounts> {
    let pdas = DevnetPdas::new()?;
    Ok(PurchaseSubscriptionTokenUsdtAccounts {
        buyer,
        backend_admin,
        usdt_mint: pdas.usdt_mint,
        buyer_usdt_account: pdas.user_usdt_ata(&buyer)?.address,
        usdt_treasury_vault: pdas.usdt_treasury_vault_ata()?.address,
        usdt_treasury_pda: pdas.usdt_treasury().address,
        subscription_token_mint: pdas.txl_mint,
        token_treasury_vault: pdas.token_treasury_vault_ata()?.address,
        token_treasury_pda: pdas.token_treasury_v2().address,
        buyer_token_account: pdas.user_txl_ata(&buyer)?.address,
        token_program: parse_pubkey(LEGACY_TOKEN_PROGRAM_ID)?,
        token_2022_program: parse_pubkey(TOKEN_2022_PROGRAM_ID)?,
        system_program: parse_pubkey(SYSTEM_PROGRAM_ID)?,
        associated_token_program: parse_pubkey(ASSOCIATED_TOKEN_PROGRAM_ID)?,
    })
}

pub fn purchase_subscription_token_usdt_instruction(
    program_id: Pubkey,
    accounts: PurchaseSubscriptionTokenUsdtAccounts,
    txline_amount: u64,
) -> Result<Instruction> {
    validate_quote_amount(txline_amount)?;
    let mut data = Vec::with_capacity(16);
    data.extend_from_slice(&PURCHASE_SUBSCRIPTION_TOKEN_USDT_DISCRIMINATOR);
    data.extend_from_slice(&txline_amount.to_le_bytes());

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.buyer, true),
            AccountMeta::new_readonly(accounts.backend_admin, true),
            AccountMeta::new_readonly(accounts.usdt_mint, false),
            AccountMeta::new(accounts.buyer_usdt_account, false),
            AccountMeta::new(accounts.usdt_treasury_vault, false),
            AccountMeta::new_readonly(accounts.usdt_treasury_pda, false),
            AccountMeta::new_readonly(accounts.subscription_token_mint, false),
            AccountMeta::new(accounts.token_treasury_vault, false),
            AccountMeta::new_readonly(accounts.token_treasury_pda, false),
            AccountMeta::new(accounts.buyer_token_account, false),
            AccountMeta::new_readonly(accounts.token_program, false),
            AccountMeta::new_readonly(accounts.token_2022_program, false),
            AccountMeta::new_readonly(accounts.system_program, false),
            AccountMeta::new_readonly(accounts.associated_token_program, false),
        ],
        data,
    })
}

impl PurchaseQuoteResponse {
    pub fn transaction_bytes(&self) -> Result<Vec<u8>> {
        let bytes = STANDARD.decode(&self.transaction_base64)?;
        if bytes.is_empty() {
            return Err(TxlineError::solana(
                "purchase quote transaction decoded to an empty byte buffer",
            ));
        }
        Ok(bytes)
    }

    pub fn validate_financial_shape(&self) -> Result<()> {
        if self.base_usdt_cost < 0.0 || self.fee_usdt_amount < 0.0 || self.total_usdt_charged < 0.0
        {
            return Err(TxlineError::solana(
                "purchase quote contains negative USDT amounts",
            ));
        }
        let expected = self.base_usdt_cost + self.fee_usdt_amount;
        if (expected - self.total_usdt_charged).abs() > 0.000_001 {
            return Err(TxlineError::solana(
                "purchase quote total does not equal base cost plus fee",
            ));
        }
        Ok(())
    }
}
