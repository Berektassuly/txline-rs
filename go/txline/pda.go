package txline

import "github.com/gagliardetto/solana-go"

type PDA struct {
	Address solana.PublicKey
	Bump    uint8
}

type DevnetPDAs struct {
	ProgramID solana.PublicKey
	TxLMint   solana.PublicKey
	USDTMint  solana.PublicKey
}

func NewDevnetPDAs() DevnetPDAs {
	return DevnetPDAs{
		ProgramID: DevnetProgramPublicKey(),
		TxLMint:   DevnetTxLMintPublicKey(),
		USDTMint:  DevnetUSDTMintPublicKey(),
	}
}

func (p DevnetPDAs) PricingMatrix() PDA {
	return findPDA([][]byte{[]byte("pricing_matrix")}, p.ProgramID)
}

func (p DevnetPDAs) TokenTreasuryV2() PDA {
	return findPDA([][]byte{[]byte("token_treasury_v2")}, p.ProgramID)
}

func (p DevnetPDAs) USDTTreasury() PDA {
	return findPDA([][]byte{[]byte("usdt_treasury")}, p.ProgramID)
}

func (p DevnetPDAs) TokenTreasuryVaultATA() (PDA, error) {
	return Token2022AssociatedTokenAddress(p.TokenTreasuryV2().Address, p.TxLMint)
}

func (p DevnetPDAs) USDTTreasuryVaultATA() (PDA, error) {
	return Token2022AssociatedTokenAddress(p.USDTTreasury().Address, p.USDTMint)
}

func (p DevnetPDAs) UserTxLATA(user solana.PublicKey) (PDA, error) {
	return Token2022AssociatedTokenAddress(user, p.TxLMint)
}

func (p DevnetPDAs) UserUSDTATA(user solana.PublicKey) (PDA, error) {
	return Token2022AssociatedTokenAddress(user, p.USDTMint)
}

func (p DevnetPDAs) DailyScoresRoots(epochDay uint16) PDA {
	day := []byte{byte(epochDay), byte(epochDay >> 8)}
	return findPDA([][]byte{[]byte("daily_scores_roots"), day}, p.ProgramID)
}

func (p DevnetPDAs) DailyBatchRoots(epochDay uint16) PDA {
	day := []byte{byte(epochDay), byte(epochDay >> 8)}
	return findPDA([][]byte{[]byte("daily_batch_roots"), day}, p.ProgramID)
}

func (p DevnetPDAs) DailyOddsMerkleRoots(epochDay uint16) PDA {
	return p.DailyBatchRoots(epochDay)
}

func (p DevnetPDAs) TenDailyFixturesRoots(epochDay uint16) PDA {
	aligned := epochDay - epochDay%10
	day := []byte{byte(aligned), byte(aligned >> 8)}
	return findPDA([][]byte{[]byte("ten_daily_fixtures_roots"), day}, p.ProgramID)
}

func Token2022AssociatedTokenAddress(owner, mint solana.PublicKey) (PDA, error) {
	address, bump, err := solana.FindAssociatedTokenAddressWithProgram(owner, mint, solana.Token2022ProgramID)
	if err != nil {
		return PDA{}, &Error{Kind: ErrSolana, Msg: "could not derive Token-2022 associated token account", Err: err}
	}
	return PDA{Address: address, Bump: bump}, nil
}

func findPDA(seeds [][]byte, programID solana.PublicKey) PDA {
	address, bump, err := solana.FindProgramAddress(seeds, programID)
	if err != nil {
		panic(err)
	}
	return PDA{Address: address, Bump: bump}
}
