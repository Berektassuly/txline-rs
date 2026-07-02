//! Conservative purchase-quote transaction safety checks.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::VersionedTransaction;

use super::pda::{
    ASSOCIATED_TOKEN_PROGRAM_ID, COMPUTE_BUDGET_PROGRAM_ID, DevnetPdas, LEGACY_TOKEN_PROGRAM_ID,
    SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, parse_pubkey,
};
use super::purchase::PURCHASE_SUBSCRIPTION_TOKEN_USDT_DISCRIMINATOR;
use crate::config::TxlineConfig;
use crate::http::models::PurchaseQuoteResponse;
use crate::{Result, TxlineError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseTransactionSafetyConfig {
    pub txline_program_id: Pubkey,
    pub expected_buyer: Pubkey,
    pub expected_txline_amount: u64,
    /// Expected backend signer for safe purchase quote verification.
    ///
    /// Safe verification rejects configurations where this is `None`. The field
    /// remains optional to preserve existing struct construction, but callers
    /// that intentionally need unbound backend inspection must use
    /// [`LowLevelPurchaseTransactionSafetyConfig`] and the explicitly low-level
    /// verifier functions.
    pub expected_backend_signer: Option<Pubkey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowLevelPurchaseTransactionSafetyConfig {
    pub txline_program_id: Pubkey,
    pub expected_buyer: Pubkey,
    pub expected_txline_amount: u64,
    pub expected_backend_signer: Option<Pubkey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseTransactionSafetyReport {
    pub fee_payer: Pubkey,
    pub invoked_programs: Vec<Pubkey>,
    pub txline_purchase_instruction_count: usize,
    pub backend_signer_present: bool,
}

/// Purchase quote that has passed SDK safety validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedPurchaseQuote {
    /// Original backend quote response retained for pricing display and audit.
    pub quote: PurchaseQuoteResponse,
    /// Safety report produced from the decoded transaction.
    pub safety_report: PurchaseTransactionSafetyReport,
    transaction_bytes: Vec<u8>,
}

impl PurchaseTransactionSafetyConfig {
    pub fn devnet(
        config: &TxlineConfig,
        expected_buyer: Pubkey,
        expected_txline_amount: u64,
        expected_backend_signer: Pubkey,
    ) -> Result<Self> {
        Ok(Self {
            txline_program_id: parse_pubkey(&config.program_id)?,
            expected_buyer,
            expected_txline_amount,
            expected_backend_signer: Some(expected_backend_signer),
        })
    }

    fn low_level_config(&self) -> Result<LowLevelPurchaseTransactionSafetyConfig> {
        let expected_backend_signer = self.expected_backend_signer.ok_or_else(|| {
            TxlineError::solana("safe purchase validation requires an expected backend signer")
        })?;
        Ok(LowLevelPurchaseTransactionSafetyConfig {
            txline_program_id: self.txline_program_id,
            expected_buyer: self.expected_buyer,
            expected_txline_amount: self.expected_txline_amount,
            expected_backend_signer: Some(expected_backend_signer),
        })
    }
}

impl LowLevelPurchaseTransactionSafetyConfig {
    /// Build a Devnet low-level purchase safety config.
    ///
    /// This API intentionally permits `expected_backend_signer: None` for
    /// compatibility with raw transaction inspection. When `None` is supplied,
    /// verification does not bind the transaction to a backend signer identity.
    /// Prefer [`PurchaseTransactionSafetyConfig::devnet`] for any path that may
    /// sign or submit a purchase quote.
    pub fn devnet_unchecked_backend_signer(
        config: &TxlineConfig,
        expected_buyer: Pubkey,
        expected_txline_amount: u64,
        expected_backend_signer: Option<Pubkey>,
    ) -> Result<Self> {
        Ok(Self {
            txline_program_id: parse_pubkey(&config.program_id)?,
            expected_buyer,
            expected_txline_amount,
            expected_backend_signer,
        })
    }
}

impl ValidatedPurchaseQuote {
    /// Validate a raw backend quote and retain the audited transaction bytes.
    pub fn new(
        quote: PurchaseQuoteResponse,
        config: &PurchaseTransactionSafetyConfig,
    ) -> Result<Self> {
        quote.validate_financial_shape()?;
        let transaction_bytes = quote.raw_transaction_bytes_unchecked()?;
        let safety_report = verify_purchase_transaction_bytes(&transaction_bytes, config)?;
        Ok(Self {
            quote,
            safety_report,
            transaction_bytes,
        })
    }

    /// Transaction bytes that passed SDK safety validation.
    pub fn transaction_bytes(&self) -> &[u8] {
        &self.transaction_bytes
    }

    /// Consume the checked quote and return the validated transaction bytes.
    pub fn into_transaction_bytes(self) -> Vec<u8> {
        self.transaction_bytes
    }
}

impl PurchaseQuoteResponse {
    pub fn validated_transaction_bytes(
        &self,
        config: &PurchaseTransactionSafetyConfig,
    ) -> Result<Vec<u8>> {
        self.validate_financial_shape()?;
        let bytes = self.raw_transaction_bytes_unchecked()?;
        verify_purchase_transaction_bytes(&bytes, config)?;
        Ok(bytes)
    }

    pub fn validate_transaction_safety(
        &self,
        config: &PurchaseTransactionSafetyConfig,
    ) -> Result<PurchaseTransactionSafetyReport> {
        self.validate_financial_shape()?;
        let bytes = self.raw_transaction_bytes_unchecked()?;
        verify_purchase_transaction_bytes(&bytes, config)
    }
}

pub fn verify_purchase_transaction_base64(
    transaction_base64: &str,
    config: &PurchaseTransactionSafetyConfig,
) -> Result<PurchaseTransactionSafetyReport> {
    let bytes = STANDARD.decode(transaction_base64)?;
    verify_purchase_transaction_bytes(&bytes, config)
}

pub fn verify_purchase_transaction_bytes(
    transaction_bytes: &[u8],
    config: &PurchaseTransactionSafetyConfig,
) -> Result<PurchaseTransactionSafetyReport> {
    let transaction = decode_versioned_transaction(transaction_bytes)?;
    verify_purchase_transaction(&transaction, config)
}

/// Low-level verifier that permits unbound backend signer inspection.
///
/// If `config.expected_backend_signer` is `None`, this function cannot prove
/// which backend signed the quote transaction. Safe signing or submission flows
/// should use [`verify_purchase_transaction_bytes`] instead.
pub fn verify_purchase_transaction_bytes_low_level_unchecked_backend_signer(
    transaction_bytes: &[u8],
    config: &LowLevelPurchaseTransactionSafetyConfig,
) -> Result<PurchaseTransactionSafetyReport> {
    let transaction = decode_versioned_transaction(transaction_bytes)?;
    verify_purchase_transaction_low_level_unchecked_backend_signer(&transaction, config)
}

pub fn verify_purchase_transaction(
    transaction: &VersionedTransaction,
    config: &PurchaseTransactionSafetyConfig,
) -> Result<PurchaseTransactionSafetyReport> {
    verify_purchase_transaction_low_level_unchecked_backend_signer(
        transaction,
        &config.low_level_config()?,
    )
}

/// Low-level verifier that permits unbound backend signer inspection.
///
/// If `config.expected_backend_signer` is `None`, this function cannot prove
/// which backend signed the quote transaction. Safe signing or submission flows
/// should use [`verify_purchase_transaction`] instead.
pub fn verify_purchase_transaction_low_level_unchecked_backend_signer(
    transaction: &VersionedTransaction,
    config: &LowLevelPurchaseTransactionSafetyConfig,
) -> Result<PurchaseTransactionSafetyReport> {
    transaction
        .sanitize()
        .map_err(|err| TxlineError::solana(format!("invalid purchase transaction: {err}")))?;

    if transaction
        .message
        .address_table_lookups()
        .is_some_and(|lookups| !lookups.is_empty())
    {
        return Err(TxlineError::solana(
            "purchase quote uses address table lookups; SDK cannot audit dynamically loaded accounts safely",
        ));
    }

    let account_keys = transaction.message.static_account_keys();
    let fee_payer = *account_keys
        .first()
        .ok_or_else(|| TxlineError::solana("purchase transaction has no fee payer"))?;
    if fee_payer != config.expected_buyer {
        return Err(TxlineError::solana(
            "purchase transaction fee payer is not the expected buyer",
        ));
    }

    let backend_signer_present = match config.expected_backend_signer {
        Some(backend) => signer_signature_present(transaction, &backend)?,
        None => false,
    };
    if !backend_signer_present && config.expected_backend_signer.is_some() {
        return Err(TxlineError::solana(
            "purchase transaction is missing the expected backend signer signature",
        ));
    }

    let allowed_programs = allowed_purchase_program_pubkeys(config.txline_program_id)?;
    let mut invoked_programs = Vec::new();
    let mut purchase_instruction_count = 0usize;

    for instruction in transaction.message.instructions() {
        let program_id = account_keys
            .get(usize::from(instruction.program_id_index))
            .copied()
            .ok_or_else(|| TxlineError::solana("purchase instruction program index is invalid"))?;
        if !allowed_programs.contains(&program_id) {
            return Err(TxlineError::solana(format!(
                "purchase transaction invokes unauthorized program {program_id}"
            )));
        }
        if !invoked_programs.contains(&program_id) {
            invoked_programs.push(program_id);
        }

        reject_unexpected_buyer_signer(
            transaction,
            program_id,
            config.txline_program_id,
            instruction.accounts.as_slice(),
        )?;

        if program_id == config.txline_program_id {
            purchase_instruction_count += 1;
            verify_purchase_instruction_data(instruction.data.as_slice(), config)?;
            verify_purchase_instruction_accounts(
                account_keys,
                instruction.accounts.as_slice(),
                config,
            )?;
        }
    }

    if purchase_instruction_count != 1 {
        return Err(TxlineError::solana(format!(
            "purchase transaction must contain exactly one TxLINE purchase instruction, found {purchase_instruction_count}"
        )));
    }

    Ok(PurchaseTransactionSafetyReport {
        fee_payer,
        invoked_programs,
        txline_purchase_instruction_count: purchase_instruction_count,
        backend_signer_present,
    })
}

pub fn allowed_purchase_programs(txline_program_id: &str) -> [&str; 6] {
    [
        txline_program_id,
        COMPUTE_BUDGET_PROGRAM_ID,
        SYSTEM_PROGRAM_ID,
        LEGACY_TOKEN_PROGRAM_ID,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
    ]
}

fn allowed_purchase_program_pubkeys(txline_program_id: Pubkey) -> Result<[Pubkey; 6]> {
    Ok([
        txline_program_id,
        parse_pubkey(COMPUTE_BUDGET_PROGRAM_ID)?,
        parse_pubkey(SYSTEM_PROGRAM_ID)?,
        parse_pubkey(LEGACY_TOKEN_PROGRAM_ID)?,
        parse_pubkey(TOKEN_2022_PROGRAM_ID)?,
        parse_pubkey(ASSOCIATED_TOKEN_PROGRAM_ID)?,
    ])
}

fn decode_versioned_transaction(transaction_bytes: &[u8]) -> Result<VersionedTransaction> {
    if transaction_bytes.is_empty() {
        return Err(TxlineError::solana(
            "purchase quote transaction decoded to an empty byte buffer",
        ));
    }
    wincode::deserialize(transaction_bytes)
        .map_err(|err| TxlineError::solana(format!("could not decode purchase transaction: {err}")))
}

fn signer_signature_present(transaction: &VersionedTransaction, signer: &Pubkey) -> Result<bool> {
    let signer_index = transaction
        .message
        .static_account_keys()
        .iter()
        .position(|key| key == signer)
        .ok_or_else(|| {
            TxlineError::solana("expected backend signer is not present in transaction accounts")
        })?;
    if !transaction.message.is_signer(signer_index) {
        return Err(TxlineError::solana(
            "expected backend signer account is not marked as a signer",
        ));
    }
    Ok(transaction
        .signatures
        .get(signer_index)
        .is_some_and(|signature| *signature != Signature::default()))
}

fn reject_unexpected_buyer_signer(
    transaction: &VersionedTransaction,
    program_id: Pubkey,
    txline_program_id: Pubkey,
    instruction_accounts: &[u8],
) -> Result<()> {
    let buyer_index = 0usize;
    let buyer_is_instruction_signer = instruction_accounts.iter().any(|index| {
        usize::from(*index) == buyer_index && transaction.message.is_signer(buyer_index)
    });
    if buyer_is_instruction_signer {
        let txline_program_allowed = program_id == txline_program_id;
        let associated_program_allowed = program_id == parse_pubkey(ASSOCIATED_TOKEN_PROGRAM_ID)?;
        if !txline_program_allowed && !associated_program_allowed {
            return Err(TxlineError::solana(format!(
                "buyer wallet is requested as signer for unauthorized program {program_id}"
            )));
        }
    }
    Ok(())
}

fn verify_purchase_instruction_data(
    data: &[u8],
    config: &LowLevelPurchaseTransactionSafetyConfig,
) -> Result<()> {
    if data.len() != 16 {
        return Err(TxlineError::solana(format!(
            "purchase instruction data length is {}, expected 16",
            data.len()
        )));
    }
    if data[..8] != PURCHASE_SUBSCRIPTION_TOKEN_USDT_DISCRIMINATOR {
        return Err(TxlineError::solana(
            "TxLINE instruction is not purchase_subscription_token_usdt",
        ));
    }
    let mut amount = [0u8; 8];
    amount.copy_from_slice(&data[8..16]);
    let amount = u64::from_le_bytes(amount);
    if amount != config.expected_txline_amount {
        return Err(TxlineError::solana(format!(
            "purchase txline_amount {amount} does not match expected {}",
            config.expected_txline_amount
        )));
    }
    Ok(())
}

fn verify_purchase_instruction_accounts(
    account_keys: &[Pubkey],
    instruction_accounts: &[u8],
    config: &LowLevelPurchaseTransactionSafetyConfig,
) -> Result<()> {
    if instruction_accounts.len() != 14 {
        return Err(TxlineError::solana(format!(
            "purchase instruction account count is {}, expected 14",
            instruction_accounts.len()
        )));
    }
    let pdas = DevnetPdas::new()?;
    let expected_backend = config.expected_backend_signer;
    let expected_accounts = [
        Some(config.expected_buyer),
        expected_backend,
        Some(pdas.usdt_mint),
        Some(pdas.user_usdt_ata(&config.expected_buyer)?.address),
        Some(pdas.usdt_treasury_vault_ata()?.address),
        Some(pdas.usdt_treasury().address),
        Some(pdas.txl_mint),
        Some(pdas.token_treasury_vault_ata()?.address),
        Some(pdas.token_treasury_v2().address),
        Some(pdas.user_txl_ata(&config.expected_buyer)?.address),
        Some(parse_pubkey(LEGACY_TOKEN_PROGRAM_ID)?),
        Some(parse_pubkey(TOKEN_2022_PROGRAM_ID)?),
        Some(parse_pubkey(SYSTEM_PROGRAM_ID)?),
        Some(parse_pubkey(ASSOCIATED_TOKEN_PROGRAM_ID)?),
    ];

    for (position, expected) in expected_accounts.iter().enumerate() {
        let actual_index = usize::from(instruction_accounts[position]);
        let actual = account_keys.get(actual_index).copied().ok_or_else(|| {
            TxlineError::solana(format!(
                "purchase instruction account index {actual_index} is invalid"
            ))
        })?;
        if let Some(expected) = expected
            && actual != *expected
        {
            return Err(TxlineError::solana(format!(
                "purchase instruction account {position} is {actual}, expected {expected}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEVNET_PROGRAM_ID;
    use crate::solana::purchase::{
        devnet_purchase_subscription_token_usdt_accounts,
        purchase_subscription_token_usdt_instruction,
    };
    use solana_sdk::hash::Hash;
    use solana_sdk::instruction::Instruction;
    use solana_sdk::signature::{Keypair, Signer};
    use solana_sdk::transaction::Transaction;

    #[test]
    fn accepts_synthetic_devnet_purchase_transaction() {
        let buyer = Keypair::new();
        let backend = Keypair::new();
        let amount = 1_000;
        let config = safety_config(&buyer, &backend, amount);
        let transaction = signed_purchase_transaction(&buyer, &backend, amount, Vec::new());
        let bytes = wincode::serialize(&transaction).unwrap();

        let report = verify_purchase_transaction_bytes(&bytes, &config).unwrap();

        assert_eq!(report.fee_payer, buyer.pubkey());
        assert_eq!(report.txline_purchase_instruction_count, 1);
        assert!(report.backend_signer_present);
    }

    #[test]
    fn rejects_purchase_amount_mismatch() {
        let buyer = Keypair::new();
        let backend = Keypair::new();
        let transaction = signed_purchase_transaction(&buyer, &backend, 1_000, Vec::new());
        let config = safety_config(&buyer, &backend, 999);

        let err = verify_purchase_transaction(&transaction, &config).unwrap_err();

        assert!(err.to_string().contains("txline_amount"));
    }

    #[test]
    fn safe_validation_requires_expected_backend_signer() {
        let buyer = Keypair::new();
        let backend = Keypair::new();
        let transaction = signed_purchase_transaction(&buyer, &backend, 1_000, Vec::new());
        let mut config = safety_config(&buyer, &backend, 1_000);
        config.expected_backend_signer = None;

        let err = verify_purchase_transaction(&transaction, &config).unwrap_err();

        assert!(
            err.to_string()
                .contains("requires an expected backend signer")
        );
    }

    #[test]
    fn low_level_unchecked_validation_can_inspect_without_backend_binding() {
        let buyer = Keypair::new();
        let backend = Keypair::new();
        let transaction = signed_purchase_transaction(&buyer, &backend, 1_000, Vec::new());
        let config = LowLevelPurchaseTransactionSafetyConfig {
            txline_program_id: parse_pubkey(DEVNET_PROGRAM_ID).unwrap(),
            expected_buyer: buyer.pubkey(),
            expected_txline_amount: 1_000,
            expected_backend_signer: None,
        };

        let report =
            verify_purchase_transaction_low_level_unchecked_backend_signer(&transaction, &config)
                .unwrap();

        assert!(!report.backend_signer_present);
        assert_eq!(report.txline_purchase_instruction_count, 1);
    }

    #[test]
    fn validated_transaction_bytes_rejects_amount_mismatch() {
        let buyer = Keypair::new();
        let backend = Keypair::new();
        let transaction = signed_purchase_transaction(&buyer, &backend, 999, Vec::new());
        let quote = quote_response(&transaction);
        let raw = quote.raw_transaction_bytes_unchecked().unwrap();
        let config = safety_config(&buyer, &backend, 1_000);

        let err = quote.validated_transaction_bytes(&config).unwrap_err();

        assert!(err.to_string().contains("txline_amount"));
        assert_eq!(raw, wincode::serialize(&transaction).unwrap());
    }

    #[test]
    fn validated_transaction_bytes_rejects_backend_account_mismatch() {
        let buyer = Keypair::new();
        let backend = Keypair::new();
        let expected_backend = Keypair::new();
        let transaction = signed_purchase_transaction(&buyer, &backend, 1_000, Vec::new());
        let quote = quote_response(&transaction);
        let config = safety_config(&buyer, &expected_backend, 1_000);

        let err = quote.validated_transaction_bytes(&config).unwrap_err();

        assert!(err.to_string().contains("expected backend signer"));
    }

    #[test]
    fn rejects_unknown_program_invocation() {
        let buyer = Keypair::new();
        let backend = Keypair::new();
        let rogue_ix = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: Vec::new(),
            data: Vec::new(),
        };
        let transaction = signed_purchase_transaction(&buyer, &backend, 1_000, vec![rogue_ix]);
        let config = safety_config(&buyer, &backend, 1_000);

        let err = verify_purchase_transaction(&transaction, &config).unwrap_err();

        assert!(err.to_string().contains("unauthorized program"));
    }

    #[test]
    fn rejects_multiple_txline_purchase_instructions() {
        let buyer = Keypair::new();
        let backend = Keypair::new();
        let program_id = parse_pubkey(DEVNET_PROGRAM_ID).unwrap();
        let accounts =
            devnet_purchase_subscription_token_usdt_accounts(buyer.pubkey(), backend.pubkey())
                .unwrap();
        let extra =
            purchase_subscription_token_usdt_instruction(program_id, accounts, 1_000).unwrap();
        let transaction = signed_purchase_transaction(&buyer, &backend, 1_000, vec![extra]);
        let config = safety_config(&buyer, &backend, 1_000);

        let err = verify_purchase_transaction(&transaction, &config).unwrap_err();

        assert!(err.to_string().contains("exactly one"));
    }

    fn safety_config(
        buyer: &Keypair,
        backend: &Keypair,
        amount: u64,
    ) -> PurchaseTransactionSafetyConfig {
        PurchaseTransactionSafetyConfig {
            txline_program_id: parse_pubkey(DEVNET_PROGRAM_ID).unwrap(),
            expected_buyer: buyer.pubkey(),
            expected_txline_amount: amount,
            expected_backend_signer: Some(backend.pubkey()),
        }
    }

    fn signed_purchase_transaction(
        buyer: &Keypair,
        backend: &Keypair,
        amount: u64,
        mut extra_instructions: Vec<Instruction>,
    ) -> VersionedTransaction {
        let program_id = parse_pubkey(DEVNET_PROGRAM_ID).unwrap();
        let accounts =
            devnet_purchase_subscription_token_usdt_accounts(buyer.pubkey(), backend.pubkey())
                .unwrap();
        let purchase_ix =
            purchase_subscription_token_usdt_instruction(program_id, accounts, amount).unwrap();
        let mut instructions = vec![purchase_ix];
        instructions.append(&mut extra_instructions);
        let blockhash = Hash::new_unique();
        let mut transaction = Transaction::new_with_payer(&instructions, Some(&buyer.pubkey()));
        transaction.sign(&[buyer, backend], blockhash);
        VersionedTransaction::from(transaction)
    }

    fn quote_response(transaction: &VersionedTransaction) -> PurchaseQuoteResponse {
        PurchaseQuoteResponse {
            transaction_base64: STANDARD.encode(wincode::serialize(transaction).unwrap()),
            base_usdt_cost: 1.0,
            fee_usdt_amount: 0.25,
            total_usdt_charged: 1.25,
        }
    }
}
