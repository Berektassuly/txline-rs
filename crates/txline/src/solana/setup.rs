//! High-level Devnet user setup flow.

use std::path::Path;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use solana_client::nonblocking::rpc_client::RpcClient as AsyncRpcClient;
use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Signature, Signer, read_keypair_file};
use solana_sdk::transaction::Transaction;

use super::pda::{
    ASSOCIATED_TOKEN_PROGRAM_ID, DevnetPdas, SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, parse_pubkey,
};
use super::subscription::send_subscribe_transaction_async;
use crate::{ApiToken, GuestJwt, Result, TxlineClient, TxlineConfig, TxlineError};

pub const PRICING_MATRIX_ACCOUNT_DISCRIMINATOR: [u8; 8] = [173, 13, 64, 22, 248, 77, 110, 106];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRow {
    pub row_id: u16,
    pub price_per_week_token: u64,
    pub sampling_interval_sec: u32,
    pub league_bundle_id: i16,
    pub market_bundle_id: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricingMatrix {
    pub admin: Pubkey,
    pub rows: Vec<ServiceRow>,
}

#[derive(Debug, Clone)]
pub struct DevnetUserSetup<'a> {
    client: &'a TxlineClient,
    service_level_id: u16,
    weeks: u8,
    selected_leagues: Vec<i32>,
    existing_guest_jwt: Option<GuestJwt>,
    existing_api_token: Option<ApiToken>,
    ata_visibility_attempts: usize,
    ata_visibility_delay: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevnetUserSetupResult {
    pub user_pubkey: Pubkey,
    pub user_txl_ata: Pubkey,
    pub subscribe_signature: Option<Signature>,
    pub guest_jwt: GuestJwt,
    pub api_token: ApiToken,
    pub pricing_matrix: PricingMatrix,
}

impl TxlineClient {
    pub fn devnet_user_setup(&self) -> DevnetUserSetup<'_> {
        DevnetUserSetup::new(self)
    }
}

impl<'a> DevnetUserSetup<'a> {
    pub fn new(client: &'a TxlineClient) -> Self {
        Self {
            client,
            service_level_id: 1,
            weeks: 4,
            selected_leagues: Vec::new(),
            existing_guest_jwt: None,
            existing_api_token: None,
            ata_visibility_attempts: 5,
            ata_visibility_delay: Duration::from_secs(2),
        }
    }

    pub fn service_level_id(mut self, service_level_id: u16) -> Self {
        self.service_level_id = service_level_id;
        self
    }

    pub fn weeks(mut self, weeks: u8) -> Self {
        self.weeks = weeks;
        self
    }

    pub fn selected_leagues(mut self, selected_leagues: impl Into<Vec<i32>>) -> Self {
        self.selected_leagues = selected_leagues.into();
        self
    }

    pub fn existing_guest_jwt(mut self, jwt: GuestJwt) -> Self {
        self.existing_guest_jwt = Some(jwt);
        self
    }

    pub fn existing_api_token(mut self, token: ApiToken) -> Self {
        self.existing_api_token = Some(token);
        self
    }

    pub fn ata_visibility_retry(mut self, attempts: usize, delay: Duration) -> Self {
        self.ata_visibility_attempts = attempts.max(1);
        self.ata_visibility_delay = delay;
        self
    }

    pub async fn run_with_keypair_path(
        self,
        path: impl AsRef<Path>,
    ) -> Result<DevnetUserSetupResult> {
        let keypair = read_keypair_file(path.as_ref())
            .map_err(|err| TxlineError::solana(format!("could not read wallet keypair: {err}")))?;
        self.run_with_signer(&keypair).await
    }

    pub async fn run_with_signer<S: Signer>(self, signer: &S) -> Result<DevnetUserSetupResult> {
        if let Some(jwt) = self.existing_guest_jwt {
            self.client.set_guest_jwt(jwt);
        }
        if let Some(token) = self.existing_api_token {
            self.client.set_api_token(token);
        }

        let pricing_matrix = fetch_devnet_pricing_matrix_async(self.client.config()).await?;
        let pdas = DevnetPdas::new()?;
        let user_pubkey = signer.pubkey();
        let user_txl_ata = pdas.user_txl_ata(&user_pubkey)?.address;

        let guest_jwt = match self.client.guest_jwt() {
            Some(jwt) => jwt,
            None => self.client.start_guest_session().await?.token,
        };

        if let Some(api_token) = self.client.api_token() {
            return Ok(DevnetUserSetupResult {
                user_pubkey,
                user_txl_ata,
                subscribe_signature: None,
                guest_jwt,
                api_token,
                pricing_matrix,
            });
        }

        ensure_user_token_2022_ata_async(
            self.client.config(),
            signer,
            &user_pubkey,
            &pdas.txl_mint,
            self.ata_visibility_attempts,
            self.ata_visibility_delay,
        )
        .await?;

        let subscribe_signature = send_subscribe_transaction_async(
            self.client.config(),
            signer,
            self.service_level_id,
            self.weeks,
        )
        .await?;

        let preimage = self
            .client
            .activation_preimage(subscribe_signature.to_string(), &self.selected_leagues)?;
        let wallet_signature = signer
            .try_sign_message(preimage.as_bytes())
            .map_err(|err| {
                TxlineError::solana(format!("could not sign activation message: {err}"))
            })?;
        let wallet_signature_base64 = STANDARD.encode(wallet_signature.as_ref());
        let api_token = self
            .client
            .activate_subscription(
                subscribe_signature.to_string(),
                &self.selected_leagues,
                wallet_signature_base64,
            )
            .await?;

        Ok(DevnetUserSetupResult {
            user_pubkey,
            user_txl_ata,
            subscribe_signature: Some(subscribe_signature),
            guest_jwt,
            api_token,
            pricing_matrix,
        })
    }
}

pub fn fetch_devnet_pricing_matrix(config: &TxlineConfig) -> Result<PricingMatrix> {
    let rpc = RpcClient::new(config.rpc_url.clone());
    let pdas = DevnetPdas::new()?;
    let data = rpc.get_account_data(&pdas.pricing_matrix().address)?;
    PricingMatrix::decode_anchor_account(&data)
}

pub async fn fetch_devnet_pricing_matrix_async(config: &TxlineConfig) -> Result<PricingMatrix> {
    let rpc = AsyncRpcClient::new(config.rpc_url.clone());
    let pdas = DevnetPdas::new()?;
    let data = rpc.get_account_data(&pdas.pricing_matrix().address).await?;
    PricingMatrix::decode_anchor_account(&data)
}

pub fn ensure_user_token_2022_ata<S: Signer>(
    config: &TxlineConfig,
    payer: &S,
    owner: &Pubkey,
    mint: &Pubkey,
    visibility_attempts: usize,
    visibility_delay: Duration,
) -> Result<Pubkey> {
    let rpc = RpcClient::new(config.rpc_url.clone());
    let ata = crate::solana::pda::token_2022_associated_token_address(owner, mint)?.address;
    if rpc.get_account(&ata).is_ok() {
        return Ok(ata);
    }

    let blockhash = rpc.get_latest_blockhash()?;
    let instruction =
        create_token_2022_associated_token_account_instruction(&payer.pubkey(), &ata, owner, mint)?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[payer], blockhash);
    rpc.send_and_confirm_transaction(&transaction)?;
    wait_for_account_visibility(&rpc, &ata, visibility_attempts, visibility_delay)?;
    Ok(ata)
}

pub async fn ensure_user_token_2022_ata_async<S: Signer>(
    config: &TxlineConfig,
    payer: &S,
    owner: &Pubkey,
    mint: &Pubkey,
    visibility_attempts: usize,
    visibility_delay: Duration,
) -> Result<Pubkey> {
    let rpc = AsyncRpcClient::new(config.rpc_url.clone());
    let ata = crate::solana::pda::token_2022_associated_token_address(owner, mint)?.address;
    if rpc.get_account(&ata).await.is_ok() {
        return Ok(ata);
    }

    let blockhash = rpc.get_latest_blockhash().await?;
    let instruction =
        create_token_2022_associated_token_account_instruction(&payer.pubkey(), &ata, owner, mint)?;
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[payer], blockhash);
    rpc.send_and_confirm_transaction(&transaction).await?;
    wait_for_account_visibility_async(&rpc, &ata, visibility_attempts, visibility_delay).await?;
    Ok(ata)
}

pub fn create_token_2022_associated_token_account_instruction(
    payer: &Pubkey,
    associated_token_account: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<Instruction> {
    Ok(Instruction {
        program_id: parse_pubkey(ASSOCIATED_TOKEN_PROGRAM_ID)?,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(*associated_token_account, false),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(parse_pubkey(SYSTEM_PROGRAM_ID)?, false),
            AccountMeta::new_readonly(parse_pubkey(TOKEN_2022_PROGRAM_ID)?, false),
        ],
        data: Vec::new(),
    })
}

fn wait_for_account_visibility(
    rpc: &RpcClient,
    account: &Pubkey,
    attempts: usize,
    delay: Duration,
) -> Result<()> {
    for attempt in 0..attempts {
        if rpc.get_account(account).is_ok() {
            return Ok(());
        }
        if attempt + 1 < attempts {
            std::thread::sleep(delay);
        }
    }
    Err(TxlineError::solana(format!(
        "token account {account} was not visible after {attempts} RPC attempts"
    )))
}

async fn wait_for_account_visibility_async(
    rpc: &AsyncRpcClient,
    account: &Pubkey,
    attempts: usize,
    delay: Duration,
) -> Result<()> {
    for attempt in 0..attempts {
        if rpc.get_account(account).await.is_ok() {
            return Ok(());
        }
        if attempt + 1 < attempts {
            tokio::time::sleep(delay).await;
        }
    }
    Err(TxlineError::solana(format!(
        "token account {account} was not visible after {attempts} RPC attempts"
    )))
}

impl PricingMatrix {
    pub fn decode_anchor_account(data: &[u8]) -> Result<Self> {
        let mut reader = PricingMatrixReader { data, offset: 0 };
        let discriminator = reader.read_array::<8>()?;
        if discriminator != PRICING_MATRIX_ACCOUNT_DISCRIMINATOR {
            return Err(TxlineError::solana(
                "pricing matrix account discriminator does not match Devnet IDL",
            ));
        }
        let admin = Pubkey::new_from_array(reader.read_array::<32>()?);
        let row_count = reader.read_u32()? as usize;
        const SERVICE_ROW_SIZE: usize = 2 + 8 + 4 + 2 + 2;
        let remaining = reader.remaining();
        if !remaining.is_multiple_of(SERVICE_ROW_SIZE) {
            return Err(TxlineError::solana(
                "pricing matrix row data length is not aligned to ServiceRow size",
            ));
        }
        let expected_rows = remaining / SERVICE_ROW_SIZE;
        if row_count != expected_rows {
            return Err(TxlineError::solana(
                "pricing matrix row count does not match account data length",
            ));
        }
        let mut rows = Vec::with_capacity(row_count);
        for _ in 0..row_count {
            rows.push(ServiceRow {
                row_id: reader.read_u16()?,
                price_per_week_token: reader.read_u64()?,
                sampling_interval_sec: reader.read_u32()?,
                league_bundle_id: reader.read_i16()?,
                market_bundle_id: reader.read_i16()?,
            });
        }
        if reader.remaining() != 0 {
            return Err(TxlineError::solana(
                "pricing matrix account has unexpected trailing bytes",
            ));
        }
        Ok(Self { admin, rows })
    }
}

struct PricingMatrixReader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> PricingMatrixReader<'a> {
    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.offset)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        if self.remaining() < N {
            return Err(TxlineError::solana(
                "pricing matrix account is shorter than the Devnet IDL layout",
            ));
        }
        let mut out = [0u8; N];
        out.copy_from_slice(&self.data[self.offset..self.offset + N]);
        self.offset += N;
        Ok(out)
    }

    fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.read_array()?))
    }

    fn read_i16(&mut self) -> Result<i16> {
        Ok(i16::from_le_bytes(self.read_array()?))
    }

    fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read_array()?))
    }

    fn read_u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.read_array()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    fn pricing_matrix_prefix(admin: Pubkey, row_count: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&PRICING_MATRIX_ACCOUNT_DISCRIMINATOR);
        data.extend_from_slice(admin.as_ref());
        data.extend_from_slice(&row_count.to_le_bytes());
        data
    }

    #[test]
    fn decodes_pricing_matrix_anchor_account() {
        let admin = Pubkey::new_unique();
        let mut data = pricing_matrix_prefix(admin, 2);
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&100u64.to_le_bytes());
        data.extend_from_slice(&30u32.to_le_bytes());
        data.extend_from_slice(&(-1i16).to_le_bytes());
        data.extend_from_slice(&2i16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&200u64.to_le_bytes());
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(&3i16.to_le_bytes());
        data.extend_from_slice(&4i16.to_le_bytes());

        let matrix = PricingMatrix::decode_anchor_account(&data).unwrap();

        assert_eq!(matrix.admin, admin);
        assert_eq!(matrix.rows.len(), 2);
        assert_eq!(matrix.rows[0].row_id, 1);
        assert_eq!(matrix.rows[0].league_bundle_id, -1);
        assert_eq!(matrix.rows[1].price_per_week_token, 200);
    }

    #[test]
    fn rejects_wrong_pricing_matrix_discriminator() {
        let data = [0u8; 8 + 32 + 4];
        let err = PricingMatrix::decode_anchor_account(&data).unwrap_err();
        assert!(err.to_string().contains("discriminator"));
    }

    #[test]
    fn rejects_pricing_matrix_huge_row_count_without_allocating() {
        let data = pricing_matrix_prefix(Pubkey::new_unique(), u32::MAX);

        let err = PricingMatrix::decode_anchor_account(&data).unwrap_err();

        assert!(err.to_string().contains("row count"));
    }

    #[test]
    fn rejects_pricing_matrix_row_count_that_exceeds_row_bytes() {
        let data = pricing_matrix_prefix(Pubkey::new_unique(), 1);

        let err = PricingMatrix::decode_anchor_account(&data).unwrap_err();

        assert!(err.to_string().contains("row count"));
    }

    #[test]
    fn rejects_pricing_matrix_unaligned_row_bytes() {
        let mut data = pricing_matrix_prefix(Pubkey::new_unique(), 0);
        data.push(0);

        let err = PricingMatrix::decode_anchor_account(&data).unwrap_err();

        assert!(err.to_string().contains("not aligned"));
    }

    #[test]
    fn token_2022_ata_create_instruction_matches_expected_programs() {
        let payer = Pubkey::new_unique();
        let ata = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let instruction =
            create_token_2022_associated_token_account_instruction(&payer, &ata, &owner, &mint)
                .unwrap();

        assert_eq!(instruction.data, Vec::<u8>::new());
        assert_eq!(instruction.accounts[0], AccountMeta::new(payer, true));
        assert_eq!(instruction.accounts[1], AccountMeta::new(ata, false));
        assert_eq!(
            instruction.accounts[2],
            AccountMeta::new_readonly(owner, false)
        );
        assert_eq!(
            instruction.accounts[3],
            AccountMeta::new_readonly(mint, false)
        );
        assert_eq!(
            instruction.accounts[5],
            AccountMeta::new_readonly(parse_pubkey(TOKEN_2022_PROGRAM_ID).unwrap(), false)
        );
    }
}
