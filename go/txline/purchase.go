package txline

import (
	"context"
	"math"

	"github.com/gagliardetto/solana-go"
)

const MaxQuoteTxlineAmount uint64 = 100_000_000

var PurchaseSubscriptionTokenUSDTDiscriminator = [8]byte{198, 251, 223, 9, 31, 184, 166, 188}

type PurchaseSubscriptionTokenUSDTAccounts struct {
	Buyer                  solana.PublicKey
	BackendAdmin           solana.PublicKey
	USDTMint               solana.PublicKey
	BuyerUSDTAccount       solana.PublicKey
	USDTTreasuryVault      solana.PublicKey
	USDTTreasuryPDA        solana.PublicKey
	SubscriptionTokenMint  solana.PublicKey
	TokenTreasuryVault     solana.PublicKey
	TokenTreasuryPDA       solana.PublicKey
	BuyerTokenAccount      solana.PublicKey
	TokenProgram           solana.PublicKey
	Token2022Program       solana.PublicKey
	SystemProgram          solana.PublicKey
	AssociatedTokenProgram solana.PublicKey
}

func (c *Client) PurchaseQuote(ctx context.Context, buyerPubkey string, txlineAmount uint64) (*PurchaseQuoteResponse, error) {
	if err := ValidateQuoteAmount(txlineAmount); err != nil {
		return nil, err
	}
	var out PurchaseQuoteResponse
	err := c.postJSON(ctx, "/guest/purchase/quote", PurchaseQuoteRequest{
		BuyerPubkey:  buyerPubkey,
		TxlineAmount: txlineAmount,
	}, false, &out)
	if err != nil {
		return nil, err
	}
	return &out, nil
}

func (c *Client) PurchaseQuoteChecked(ctx context.Context, buyer solana.PublicKey, txlineAmount uint64, expectedBackendSigner solana.PublicKey) (*ValidatedPurchaseQuote, error) {
	quote, err := c.PurchaseQuote(ctx, buyer.String(), txlineAmount)
	if err != nil {
		return nil, err
	}
	config := PurchaseTransactionSafetyConfig{
		TxlineProgramID:       DevnetProgramPublicKey(),
		ExpectedBuyer:         buyer,
		ExpectedTxlineAmount:  txlineAmount,
		ExpectedBackendSigner: &expectedBackendSigner,
	}
	return NewValidatedPurchaseQuote(*quote, config)
}

func ValidateQuoteAmount(txlineAmount uint64) error {
	if txlineAmount == 0 || txlineAmount > MaxQuoteTxlineAmount {
		return &Error{Kind: ErrInvalidInput, Msg: "txlineAmount must be 1..=100000000"}
	}
	return nil
}

func DevnetPurchaseSubscriptionTokenUSDTAccounts(buyer, backendAdmin solana.PublicKey) (PurchaseSubscriptionTokenUSDTAccounts, error) {
	pdas := NewDevnetPDAs()
	buyerUSDT, err := pdas.UserUSDTATA(buyer)
	if err != nil {
		return PurchaseSubscriptionTokenUSDTAccounts{}, err
	}
	usdtVault, err := pdas.USDTTreasuryVaultATA()
	if err != nil {
		return PurchaseSubscriptionTokenUSDTAccounts{}, err
	}
	tokenVault, err := pdas.TokenTreasuryVaultATA()
	if err != nil {
		return PurchaseSubscriptionTokenUSDTAccounts{}, err
	}
	buyerTxL, err := pdas.UserTxLATA(buyer)
	if err != nil {
		return PurchaseSubscriptionTokenUSDTAccounts{}, err
	}
	return PurchaseSubscriptionTokenUSDTAccounts{
		Buyer:                  buyer,
		BackendAdmin:           backendAdmin,
		USDTMint:               pdas.USDTMint,
		BuyerUSDTAccount:       buyerUSDT.Address,
		USDTTreasuryVault:      usdtVault.Address,
		USDTTreasuryPDA:        pdas.USDTTreasury().Address,
		SubscriptionTokenMint:  pdas.TxLMint,
		TokenTreasuryVault:     tokenVault.Address,
		TokenTreasuryPDA:       pdas.TokenTreasuryV2().Address,
		BuyerTokenAccount:      buyerTxL.Address,
		TokenProgram:           solana.TokenProgramID,
		Token2022Program:       solana.Token2022ProgramID,
		SystemProgram:          solana.SystemProgramID,
		AssociatedTokenProgram: solana.SPLAssociatedTokenAccountProgramID,
	}, nil
}

func PurchaseSubscriptionTokenUSDTInstruction(programID solana.PublicKey, accounts PurchaseSubscriptionTokenUSDTAccounts, txlineAmount uint64) (solana.Instruction, error) {
	if err := ValidateQuoteAmount(txlineAmount); err != nil {
		return nil, err
	}
	data := make([]byte, 0, 16)
	data = append(data, PurchaseSubscriptionTokenUSDTDiscriminator[:]...)
	data = appendU64(data, txlineAmount)
	return newInstruction(programID, solana.AccountMetaSlice{
		writableSigner(accounts.Buyer),
		readonlySigner(accounts.BackendAdmin),
		readonly(accounts.USDTMint),
		writable(accounts.BuyerUSDTAccount),
		writable(accounts.USDTTreasuryVault),
		readonly(accounts.USDTTreasuryPDA),
		readonly(accounts.SubscriptionTokenMint),
		writable(accounts.TokenTreasuryVault),
		readonly(accounts.TokenTreasuryPDA),
		writable(accounts.BuyerTokenAccount),
		readonly(accounts.TokenProgram),
		readonly(accounts.Token2022Program),
		readonly(accounts.SystemProgram),
		readonly(accounts.AssociatedTokenProgram),
	}, data), nil
}

func (p PurchaseQuoteResponse) RawTransactionBytesUnchecked() ([]byte, error) {
	return decodeQuoteTransactionBase64(p.TransactionBase64)
}

func (p PurchaseQuoteResponse) ValidateFinancialShape() error {
	if p.BaseUSDTCost < 0 || p.FeeUSDTAmount < 0 || p.TotalUSDTCharged < 0 {
		return &Error{Kind: ErrSolana, Msg: "purchase quote contains negative USDT amounts"}
	}
	expected := p.BaseUSDTCost + p.FeeUSDTAmount
	if math.Abs(expected-p.TotalUSDTCharged) > 0.000_001 {
		return &Error{Kind: ErrSolana, Msg: "purchase quote total does not equal base cost plus fee"}
	}
	return nil
}

func (p PurchaseQuoteResponse) ValidatedTransactionBytes(config PurchaseTransactionSafetyConfig) ([]byte, error) {
	if err := p.ValidateFinancialShape(); err != nil {
		return nil, err
	}
	bytes, err := p.RawTransactionBytesUnchecked()
	if err != nil {
		return nil, err
	}
	if _, err := VerifyPurchaseTransactionBytes(bytes, config); err != nil {
		return nil, err
	}
	return bytes, nil
}

func (p PurchaseQuoteResponse) ValidateTransactionSafety(config PurchaseTransactionSafetyConfig) (PurchaseTransactionSafetyReport, error) {
	if err := p.ValidateFinancialShape(); err != nil {
		return PurchaseTransactionSafetyReport{}, err
	}
	bytes, err := p.RawTransactionBytesUnchecked()
	if err != nil {
		return PurchaseTransactionSafetyReport{}, err
	}
	return VerifyPurchaseTransactionBytes(bytes, config)
}
