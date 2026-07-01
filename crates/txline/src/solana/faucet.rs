//! Devnet faucet instruction builder.

use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;

use super::pda::{
    ASSOCIATED_TOKEN_PROGRAM_ID, DevnetPdas, LEGACY_TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    parse_pubkey,
};
use crate::Result;

pub const REQUEST_DEVNET_FAUCET_DISCRIMINATOR: [u8; 8] = [49, 178, 104, 8, 23, 120, 186, 21];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestDevnetFaucetAccounts {
    pub user: Pubkey,
    pub faucet_tracker: Pubkey,
    pub usdt_mint: Pubkey,
    pub user_usdt_ata: Pubkey,
    pub usdt_treasury_pda: Pubkey,
    pub token_program: Pubkey,
    pub associated_token_program: Pubkey,
    pub system_program: Pubkey,
}

pub fn devnet_request_faucet_accounts(
    user: Pubkey,
    faucet_tracker: Pubkey,
) -> Result<RequestDevnetFaucetAccounts> {
    let pdas = DevnetPdas::new()?;
    Ok(RequestDevnetFaucetAccounts {
        user,
        faucet_tracker,
        usdt_mint: pdas.usdt_mint,
        user_usdt_ata: pdas.user_usdt_ata(&user)?.address,
        usdt_treasury_pda: pdas.usdt_treasury().address,
        token_program: parse_pubkey(LEGACY_TOKEN_PROGRAM_ID)?,
        associated_token_program: parse_pubkey(ASSOCIATED_TOKEN_PROGRAM_ID)?,
        system_program: parse_pubkey(SYSTEM_PROGRAM_ID)?,
    })
}

pub fn request_devnet_faucet_instruction(
    program_id: Pubkey,
    accounts: RequestDevnetFaucetAccounts,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(accounts.user, true),
            AccountMeta::new(accounts.faucet_tracker, false),
            AccountMeta::new(accounts.usdt_mint, false),
            AccountMeta::new(accounts.user_usdt_ata, false),
            AccountMeta::new_readonly(accounts.usdt_treasury_pda, false),
            AccountMeta::new_readonly(accounts.token_program, false),
            AccountMeta::new_readonly(accounts.associated_token_program, false),
            AccountMeta::new_readonly(accounts.system_program, false),
        ],
        data: REQUEST_DEVNET_FAUCET_DISCRIMINATOR.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faucet_instruction_uses_devnet_discriminator() {
        let program_id = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let tracker = Pubkey::new_unique();
        let accounts = devnet_request_faucet_accounts(user, tracker).unwrap();
        let instruction = request_devnet_faucet_instruction(program_id, accounts);

        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.data, REQUEST_DEVNET_FAUCET_DISCRIMINATOR);
        assert_eq!(instruction.accounts[0], AccountMeta::new(user, true));
        assert_eq!(instruction.accounts[1], AccountMeta::new(tracker, false));
    }
}
