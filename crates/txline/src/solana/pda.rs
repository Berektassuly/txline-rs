//! Devnet PDA helpers.

use solana_sdk::pubkey::Pubkey;

use crate::config::{DEVNET_PROGRAM_ID, DEVNET_TXL_MINT, DEVNET_USDT_MINT};
use crate::{Result, TxlineError};

pub const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
pub const LEGACY_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";
pub const COMPUTE_BUDGET_PROGRAM_ID: &str = "ComputeBudget111111111111111111111111111111";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pda {
    pub address: Pubkey,
    pub bump: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct DevnetPdas {
    pub program_id: Pubkey,
    pub txl_mint: Pubkey,
    pub usdt_mint: Pubkey,
}

impl DevnetPdas {
    pub fn new() -> Result<Self> {
        Ok(Self {
            program_id: parse_pubkey(DEVNET_PROGRAM_ID)?,
            txl_mint: parse_pubkey(DEVNET_TXL_MINT)?,
            usdt_mint: parse_pubkey(DEVNET_USDT_MINT)?,
        })
    }

    pub fn pricing_matrix(&self) -> Pda {
        find_pda(&[b"pricing_matrix"], &self.program_id)
    }

    pub fn token_treasury_v2(&self) -> Pda {
        find_pda(&[b"token_treasury_v2"], &self.program_id)
    }

    pub fn usdt_treasury(&self) -> Pda {
        find_pda(&[b"usdt_treasury"], &self.program_id)
    }

    pub fn token_treasury_vault_ata(&self) -> Result<Pda> {
        token_2022_associated_token_address(&self.token_treasury_v2().address, &self.txl_mint)
    }

    pub fn usdt_treasury_vault_ata(&self) -> Result<Pda> {
        token_2022_associated_token_address(&self.usdt_treasury().address, &self.usdt_mint)
    }

    pub fn user_txl_ata(&self, user: &Pubkey) -> Result<Pda> {
        token_2022_associated_token_address(user, &self.txl_mint)
    }

    pub fn user_usdt_ata(&self, user: &Pubkey) -> Result<Pda> {
        token_2022_associated_token_address(user, &self.usdt_mint)
    }

    pub fn daily_scores_roots(&self, epoch_day: u16) -> Pda {
        let day = epoch_day.to_le_bytes();
        find_pda(&[b"daily_scores_roots", &day], &self.program_id)
    }

    pub fn daily_batch_roots(&self, epoch_day: u16) -> Pda {
        let day = epoch_day.to_le_bytes();
        find_pda(&[b"daily_batch_roots", &day], &self.program_id)
    }

    /// PDA used by the Devnet IDL account `daily_odds_merkle_roots`.
    ///
    /// Upstream docs derive odds validation roots with the `daily_batch_roots`
    /// seed, even though the validation instruction account is named
    /// `daily_odds_merkle_roots`.
    pub fn daily_odds_merkle_roots(&self, epoch_day: u16) -> Pda {
        self.daily_batch_roots(epoch_day)
    }

    pub fn ten_daily_fixtures_roots(&self, epoch_day: u16) -> Pda {
        let aligned = epoch_day - (epoch_day % 10);
        let day = aligned.to_le_bytes();
        find_pda(&[b"ten_daily_fixtures_roots", &day], &self.program_id)
    }
}

pub fn token_2022_associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Result<Pda> {
    let token_program = parse_pubkey(TOKEN_2022_PROGRAM_ID)?;
    let associated_program = parse_pubkey(ASSOCIATED_TOKEN_PROGRAM_ID)?;
    Ok(find_pda(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &associated_program,
    ))
}

pub fn parse_pubkey(value: &str) -> Result<Pubkey> {
    value
        .parse::<Pubkey>()
        .map_err(|err| TxlineError::solana(format!("invalid pubkey {value}: {err}")))
}

fn find_pda(seeds: &[&[u8]], program_id: &Pubkey) -> Pda {
    let (address, bump) = Pubkey::find_program_address(seeds, program_id);
    Pda { address, bump }
}
