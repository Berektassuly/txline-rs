package txline

import (
	"encoding/base64"
	"fmt"

	"github.com/gagliardetto/solana-go"
)

type PurchaseTransactionSafetyConfig struct {
	TxlineProgramID       solana.PublicKey
	ExpectedBuyer         solana.PublicKey
	ExpectedTxlineAmount  uint64
	ExpectedBackendSigner *solana.PublicKey
}

type LowLevelPurchaseTransactionSafetyConfig struct {
	TxlineProgramID       solana.PublicKey
	ExpectedBuyer         solana.PublicKey
	ExpectedTxlineAmount  uint64
	ExpectedBackendSigner *solana.PublicKey
}

type PurchaseTransactionSafetyReport struct {
	FeePayer                       solana.PublicKey
	InvokedPrograms                []solana.PublicKey
	TxlinePurchaseInstructionCount int
	BackendSignerPresent           bool
}

type ValidatedPurchaseQuote struct {
	Quote            PurchaseQuoteResponse
	SafetyReport     PurchaseTransactionSafetyReport
	transactionBytes []byte
}

func DevnetPurchaseTransactionSafetyConfig(expectedBuyer solana.PublicKey, expectedTxlineAmount uint64, expectedBackendSigner solana.PublicKey) PurchaseTransactionSafetyConfig {
	return PurchaseTransactionSafetyConfig{
		TxlineProgramID:       DevnetProgramPublicKey(),
		ExpectedBuyer:         expectedBuyer,
		ExpectedTxlineAmount:  expectedTxlineAmount,
		ExpectedBackendSigner: &expectedBackendSigner,
	}
}

func NewValidatedPurchaseQuote(quote PurchaseQuoteResponse, config PurchaseTransactionSafetyConfig) (*ValidatedPurchaseQuote, error) {
	if err := quote.ValidateFinancialShape(); err != nil {
		return nil, err
	}
	transactionBytes, err := quote.RawTransactionBytesUnchecked()
	if err != nil {
		return nil, err
	}
	report, err := VerifyPurchaseTransactionBytes(transactionBytes, config)
	if err != nil {
		return nil, err
	}
	return &ValidatedPurchaseQuote{
		Quote:            quote,
		SafetyReport:     report,
		transactionBytes: transactionBytes,
	}, nil
}

func (v ValidatedPurchaseQuote) TransactionBytes() []byte {
	return append([]byte(nil), v.transactionBytes...)
}

func (v ValidatedPurchaseQuote) IntoTransactionBytes() []byte {
	return append([]byte(nil), v.transactionBytes...)
}

func VerifyPurchaseTransactionBase64(transactionBase64 string, config PurchaseTransactionSafetyConfig) (PurchaseTransactionSafetyReport, error) {
	bytes, err := base64.StdEncoding.DecodeString(transactionBase64)
	if err != nil {
		return PurchaseTransactionSafetyReport{}, &Error{Kind: ErrSolana, Msg: "could not decode purchase transaction", Err: err}
	}
	return VerifyPurchaseTransactionBytes(bytes, config)
}

func VerifyPurchaseTransactionBytes(transactionBytes []byte, config PurchaseTransactionSafetyConfig) (PurchaseTransactionSafetyReport, error) {
	tx, err := decodePurchaseTransaction(transactionBytes)
	if err != nil {
		return PurchaseTransactionSafetyReport{}, err
	}
	if config.ExpectedBackendSigner == nil {
		return PurchaseTransactionSafetyReport{}, &Error{Kind: ErrSolana, Msg: "safe purchase validation requires an expected backend signer"}
	}
	low := LowLevelPurchaseTransactionSafetyConfig{
		TxlineProgramID:       config.TxlineProgramID,
		ExpectedBuyer:         config.ExpectedBuyer,
		ExpectedTxlineAmount:  config.ExpectedTxlineAmount,
		ExpectedBackendSigner: config.ExpectedBackendSigner,
	}
	return VerifyPurchaseTransactionLowLevelUncheckedBackendSigner(tx, low)
}

func VerifyPurchaseTransactionBytesLowLevelUncheckedBackendSigner(transactionBytes []byte, config LowLevelPurchaseTransactionSafetyConfig) (PurchaseTransactionSafetyReport, error) {
	tx, err := decodePurchaseTransaction(transactionBytes)
	if err != nil {
		return PurchaseTransactionSafetyReport{}, err
	}
	return VerifyPurchaseTransactionLowLevelUncheckedBackendSigner(tx, config)
}

func VerifyPurchaseTransaction(tx *solana.Transaction, config PurchaseTransactionSafetyConfig) (PurchaseTransactionSafetyReport, error) {
	if config.ExpectedBackendSigner == nil {
		return PurchaseTransactionSafetyReport{}, &Error{Kind: ErrSolana, Msg: "safe purchase validation requires an expected backend signer"}
	}
	low := LowLevelPurchaseTransactionSafetyConfig{
		TxlineProgramID:       config.TxlineProgramID,
		ExpectedBuyer:         config.ExpectedBuyer,
		ExpectedTxlineAmount:  config.ExpectedTxlineAmount,
		ExpectedBackendSigner: config.ExpectedBackendSigner,
	}
	return VerifyPurchaseTransactionLowLevelUncheckedBackendSigner(tx, low)
}

func VerifyPurchaseTransactionLowLevelUncheckedBackendSigner(tx *solana.Transaction, config LowLevelPurchaseTransactionSafetyConfig) (PurchaseTransactionSafetyReport, error) {
	if tx == nil {
		return PurchaseTransactionSafetyReport{}, &Error{Kind: ErrSolana, Msg: "purchase transaction is nil"}
	}
	if err := tx.Sanitize(); err != nil {
		return PurchaseTransactionSafetyReport{}, &Error{Kind: ErrSolana, Msg: "invalid purchase transaction", Err: err}
	}
	if tx.Message.GetAddressTableLookups().NumLookups() != 0 || len(tx.Message.GetAddressTableLookups()) != 0 {
		return PurchaseTransactionSafetyReport{}, &Error{Kind: ErrSolana, Msg: "purchase quote uses address table lookups; SDK cannot audit dynamically loaded accounts safely"}
	}
	accountKeys := tx.Message.AccountKeys
	if len(accountKeys) == 0 {
		return PurchaseTransactionSafetyReport{}, &Error{Kind: ErrSolana, Msg: "purchase transaction has no fee payer"}
	}
	feePayer := accountKeys[0]
	if feePayer != config.ExpectedBuyer {
		return PurchaseTransactionSafetyReport{}, &Error{Kind: ErrSolana, Msg: "purchase transaction fee payer is not the expected buyer"}
	}

	backendSignerPresent := false
	if config.ExpectedBackendSigner != nil {
		present, err := signerSignaturePresent(tx, *config.ExpectedBackendSigner)
		if err != nil {
			return PurchaseTransactionSafetyReport{}, err
		}
		if !present {
			return PurchaseTransactionSafetyReport{}, &Error{Kind: ErrSolana, Msg: "purchase transaction is missing the expected backend signer signature"}
		}
		backendSignerPresent = true
	}

	allowedPrograms := allowedPurchaseProgramSet(config.TxlineProgramID)
	var invoked []solana.PublicKey
	purchaseCount := 0
	for _, instruction := range tx.Message.Instructions {
		if int(instruction.ProgramIDIndex) >= len(accountKeys) {
			return PurchaseTransactionSafetyReport{}, &Error{Kind: ErrSolana, Msg: "purchase instruction program index is invalid"}
		}
		programID := accountKeys[instruction.ProgramIDIndex]
		if !allowedPrograms[programID] {
			return PurchaseTransactionSafetyReport{}, &Error{Kind: ErrSolana, Msg: fmt.Sprintf("purchase transaction invokes unauthorized program %s", programID)}
		}
		if !containsPublicKey(invoked, programID) {
			invoked = append(invoked, programID)
		}
		if err := rejectUnexpectedBuyerSigner(tx, programID, config.TxlineProgramID, instruction.Accounts); err != nil {
			return PurchaseTransactionSafetyReport{}, err
		}
		if programID == config.TxlineProgramID {
			purchaseCount++
			if err := verifyPurchaseInstructionData([]byte(instruction.Data), config.ExpectedTxlineAmount); err != nil {
				return PurchaseTransactionSafetyReport{}, err
			}
			if err := verifyPurchaseInstructionAccounts(accountKeys, instruction.Accounts, config); err != nil {
				return PurchaseTransactionSafetyReport{}, err
			}
		}
	}
	if purchaseCount != 1 {
		return PurchaseTransactionSafetyReport{}, &Error{Kind: ErrSolana, Msg: fmt.Sprintf("purchase transaction must contain exactly one TxLINE purchase instruction, found %d", purchaseCount)}
	}
	return PurchaseTransactionSafetyReport{
		FeePayer:                       feePayer,
		InvokedPrograms:                invoked,
		TxlinePurchaseInstructionCount: purchaseCount,
		BackendSignerPresent:           backendSignerPresent,
	}, nil
}

func AllowedPurchasePrograms(txlineProgramID string) [6]string {
	return [6]string{
		txlineProgramID,
		ComputeBudgetProgramIDString,
		SystemProgramIDString,
		LegacyTokenProgramIDString,
		Token2022ProgramIDString,
		AssociatedTokenProgramIDString,
	}
}

func decodePurchaseTransaction(transactionBytes []byte) (*solana.Transaction, error) {
	if len(transactionBytes) == 0 {
		return nil, &Error{Kind: ErrSolana, Msg: "purchase quote transaction decoded to an empty byte buffer"}
	}
	tx, err := solana.TransactionFromBytes(transactionBytes)
	if err != nil {
		return nil, &Error{Kind: ErrSolana, Msg: "could not decode purchase transaction", Err: err}
	}
	return tx, nil
}

func signerSignaturePresent(tx *solana.Transaction, signer solana.PublicKey) (bool, error) {
	signerIndex := -1
	for idx, key := range tx.Message.AccountKeys {
		if key == signer {
			signerIndex = idx
			break
		}
	}
	if signerIndex < 0 {
		return false, &Error{Kind: ErrSolana, Msg: "expected backend signer is not present in transaction accounts"}
	}
	if !tx.Message.IsSigner(signer) {
		return false, &Error{Kind: ErrSolana, Msg: "expected backend signer account is not marked as a signer"}
	}
	if signerIndex >= len(tx.Signatures) {
		return false, nil
	}
	var zero solana.Signature
	if tx.Signatures[signerIndex] == zero {
		return false, nil
	}
	results, err := tx.VerifyWithResults()
	if err != nil {
		return false, &Error{Kind: ErrSolana, Msg: "could not verify purchase transaction signatures", Err: err}
	}
	if signerIndex >= len(results) || !results[signerIndex] {
		return false, &Error{Kind: ErrSolana, Msg: "expected backend signer signature does not verify"}
	}
	return true, nil
}

func rejectUnexpectedBuyerSigner(tx *solana.Transaction, programID, txlineProgramID solana.PublicKey, instructionAccounts []uint16) error {
	buyerIsInstructionSigner := false
	for _, accountIndex := range instructionAccounts {
		if accountIndex == 0 && tx.Message.IsSigner(tx.Message.AccountKeys[0]) {
			buyerIsInstructionSigner = true
			break
		}
	}
	if !buyerIsInstructionSigner {
		return nil
	}
	if programID == txlineProgramID || programID == solana.SPLAssociatedTokenAccountProgramID {
		return nil
	}
	return &Error{Kind: ErrSolana, Msg: fmt.Sprintf("buyer wallet is requested as signer for unauthorized program %s", programID)}
}

func verifyPurchaseInstructionData(data []byte, expectedAmount uint64) error {
	if len(data) != 16 {
		return &Error{Kind: ErrSolana, Msg: fmt.Sprintf("purchase instruction data length is %d, expected 16", len(data))}
	}
	if string(data[:8]) != string(PurchaseSubscriptionTokenUSDTDiscriminator[:]) {
		return &Error{Kind: ErrSolana, Msg: "TxLINE instruction is not purchase_subscription_token_usdt"}
	}
	amount := binaryLEUint64(data[8:16])
	if amount != expectedAmount {
		return &Error{Kind: ErrSolana, Msg: fmt.Sprintf("purchase txline_amount %d does not match expected %d", amount, expectedAmount)}
	}
	return nil
}

func verifyPurchaseInstructionAccounts(accountKeys []solana.PublicKey, instructionAccounts []uint16, config LowLevelPurchaseTransactionSafetyConfig) error {
	if len(instructionAccounts) != 14 {
		return &Error{Kind: ErrSolana, Msg: fmt.Sprintf("purchase instruction account count is %d, expected 14", len(instructionAccounts))}
	}
	pdas := NewDevnetPDAs()
	buyerUSDT, err := pdas.UserUSDTATA(config.ExpectedBuyer)
	if err != nil {
		return err
	}
	usdtVault, err := pdas.USDTTreasuryVaultATA()
	if err != nil {
		return err
	}
	tokenVault, err := pdas.TokenTreasuryVaultATA()
	if err != nil {
		return err
	}
	buyerTxL, err := pdas.UserTxLATA(config.ExpectedBuyer)
	if err != nil {
		return err
	}
	expected := []*solana.PublicKey{
		&config.ExpectedBuyer,
		config.ExpectedBackendSigner,
		&pdas.USDTMint,
		&buyerUSDT.Address,
		&usdtVault.Address,
		ptrPublicKey(pdas.USDTTreasury().Address),
		&pdas.TxLMint,
		&tokenVault.Address,
		ptrPublicKey(pdas.TokenTreasuryV2().Address),
		&buyerTxL.Address,
		ptrPublicKey(solana.TokenProgramID),
		ptrPublicKey(solana.Token2022ProgramID),
		ptrPublicKey(solana.SystemProgramID),
		ptrPublicKey(solana.SPLAssociatedTokenAccountProgramID),
	}
	for position, expectedKey := range expected {
		actualIndex := int(instructionAccounts[position])
		if actualIndex >= len(accountKeys) {
			return &Error{Kind: ErrSolana, Msg: fmt.Sprintf("purchase instruction account index %d is invalid", actualIndex)}
		}
		if expectedKey != nil && accountKeys[actualIndex] != *expectedKey {
			return &Error{Kind: ErrSolana, Msg: fmt.Sprintf("purchase instruction account %d is %s, expected %s", position, accountKeys[actualIndex], *expectedKey)}
		}
	}
	return nil
}

func allowedPurchaseProgramSet(txlineProgramID solana.PublicKey) map[solana.PublicKey]bool {
	return map[solana.PublicKey]bool{
		txlineProgramID:                           true,
		solana.ComputeBudget:                      true,
		solana.SystemProgramID:                    true,
		solana.TokenProgramID:                     true,
		solana.Token2022ProgramID:                 true,
		solana.SPLAssociatedTokenAccountProgramID: true,
	}
}

func containsPublicKey(values []solana.PublicKey, target solana.PublicKey) bool {
	for _, value := range values {
		if value == target {
			return true
		}
	}
	return false
}

func ptrPublicKey(value solana.PublicKey) *solana.PublicKey {
	return &value
}

func binaryLEUint64(value []byte) uint64 {
	return uint64(value[0]) |
		uint64(value[1])<<8 |
		uint64(value[2])<<16 |
		uint64(value[3])<<24 |
		uint64(value[4])<<32 |
		uint64(value[5])<<40 |
		uint64(value[6])<<48 |
		uint64(value[7])<<56
}
