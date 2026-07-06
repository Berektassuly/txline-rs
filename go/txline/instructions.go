package txline

import (
	"encoding/binary"
	"fmt"
	"math"

	"github.com/gagliardetto/solana-go"
)

var (
	SubscribeDiscriminator            = [8]byte{254, 28, 191, 138, 156, 179, 183, 53}
	RequestDevnetFaucetDiscriminator  = [8]byte{49, 178, 104, 8, 23, 120, 186, 21}
	ValidateFixtureDiscriminator      = [8]byte{231, 129, 218, 86, 223, 114, 21, 126}
	ValidateFixtureBatchDiscriminator = [8]byte{85, 223, 204, 7, 4, 87, 157, 1}
	ValidateOddsDiscriminator         = [8]byte{192, 19, 91, 138, 104, 100, 212, 86}
	ValidateStatDiscriminator         = [8]byte{107, 197, 232, 90, 191, 136, 105, 185}
	ValidateStatV2Discriminator       = [8]byte{208, 215, 194, 214, 241, 71, 246, 178}
	CreateIntentDiscriminator         = [8]byte{216, 214, 79, 121, 23, 194, 96, 104}
	CreateTradeDiscriminator          = [8]byte{183, 82, 24, 245, 248, 30, 204, 246}
	ExecuteMatchDiscriminator         = [8]byte{76, 47, 91, 223, 20, 10, 147, 232}
	CloseIntentDiscriminator          = [8]byte{112, 245, 154, 249, 57, 126, 54, 122}
	SettleTradeDiscriminator          = [8]byte{252, 176, 98, 248, 73, 123, 8, 157}
	SettleMatchedTradeDiscriminator   = [8]byte{191, 233, 149, 116, 32, 239, 18, 65}
	ClaimViaResolutionDiscriminator   = [8]byte{98, 206, 250, 87, 151, 135, 162, 181}
	ClaimBatchLegacyDiscriminator     = [8]byte{254, 101, 89, 255, 169, 75, 207, 66}
	RefundBatchDiscriminator          = [8]byte{227, 54, 194, 2, 78, 8, 104, 29}
	AuditTradeResultDiscriminator     = [8]byte{50, 242, 243, 5, 209, 75, 76, 91}
)

type SubscribeAccounts struct {
	User                   solana.PublicKey
	PricingMatrix          solana.PublicKey
	TokenMint              solana.PublicKey
	UserTokenAccount       solana.PublicKey
	TokenTreasuryVault     solana.PublicKey
	TokenTreasuryPDA       solana.PublicKey
	TokenProgram           solana.PublicKey
	SystemProgram          solana.PublicKey
	AssociatedTokenProgram solana.PublicKey
}

type SubscribeParams struct {
	ServiceLevelID uint16
	Weeks          uint8
}

func ValidateSubscriptionWeeks(weeks uint8) error {
	if weeks < 4 || weeks%4 != 0 {
		return &Error{Kind: ErrInvalidInput, Msg: "subscription duration must be at least 4 weeks and a multiple of 4"}
	}
	return nil
}

func DevnetSubscribeAccounts(user solana.PublicKey) (SubscribeAccounts, error) {
	pdas := NewDevnetPDAs()
	userATA, err := pdas.UserTxLATA(user)
	if err != nil {
		return SubscribeAccounts{}, err
	}
	vault, err := pdas.TokenTreasuryVaultATA()
	if err != nil {
		return SubscribeAccounts{}, err
	}
	return SubscribeAccounts{
		User:                   user,
		PricingMatrix:          pdas.PricingMatrix().Address,
		TokenMint:              pdas.TxLMint,
		UserTokenAccount:       userATA.Address,
		TokenTreasuryVault:     vault.Address,
		TokenTreasuryPDA:       pdas.TokenTreasuryV2().Address,
		TokenProgram:           solana.Token2022ProgramID,
		SystemProgram:          solana.SystemProgramID,
		AssociatedTokenProgram: solana.SPLAssociatedTokenAccountProgramID,
	}, nil
}

func SubscribeInstruction(programID solana.PublicKey, accounts SubscribeAccounts, params SubscribeParams) (solana.Instruction, error) {
	if err := ValidateSubscriptionWeeks(params.Weeks); err != nil {
		return nil, err
	}
	data := append([]byte{}, SubscribeDiscriminator[:]...)
	data = appendU16(data, params.ServiceLevelID)
	data = append(data, params.Weeks)
	return newInstruction(programID, solana.AccountMetaSlice{
		writableSigner(accounts.User),
		readonly(accounts.PricingMatrix),
		readonly(accounts.TokenMint),
		writable(accounts.UserTokenAccount),
		writable(accounts.TokenTreasuryVault),
		readonly(accounts.TokenTreasuryPDA),
		readonly(accounts.TokenProgram),
		readonly(accounts.SystemProgram),
		readonly(accounts.AssociatedTokenProgram),
	}, data), nil
}

func CreateToken2022AssociatedTokenAccountInstruction(payer, associatedTokenAccount, owner, mint solana.PublicKey) solana.Instruction {
	return newInstruction(solana.SPLAssociatedTokenAccountProgramID, solana.AccountMetaSlice{
		writableSigner(payer),
		writable(associatedTokenAccount),
		readonly(owner),
		readonly(mint),
		readonly(solana.SystemProgramID),
		readonly(solana.Token2022ProgramID),
	}, nil)
}

type RequestDevnetFaucetAccounts struct {
	User                   solana.PublicKey
	FaucetTracker          solana.PublicKey
	USDTMint               solana.PublicKey
	UserUSDTATA            solana.PublicKey
	USDTTreasuryPDA        solana.PublicKey
	TokenProgram           solana.PublicKey
	AssociatedTokenProgram solana.PublicKey
	SystemProgram          solana.PublicKey
}

func DevnetRequestFaucetAccounts(user, faucetTracker solana.PublicKey) (RequestDevnetFaucetAccounts, error) {
	pdas := NewDevnetPDAs()
	userUSDT, err := pdas.UserUSDTATA(user)
	if err != nil {
		return RequestDevnetFaucetAccounts{}, err
	}
	return RequestDevnetFaucetAccounts{
		User:                   user,
		FaucetTracker:          faucetTracker,
		USDTMint:               pdas.USDTMint,
		UserUSDTATA:            userUSDT.Address,
		USDTTreasuryPDA:        pdas.USDTTreasury().Address,
		TokenProgram:           solana.TokenProgramID,
		AssociatedTokenProgram: solana.SPLAssociatedTokenAccountProgramID,
		SystemProgram:          solana.SystemProgramID,
	}, nil
}

func RequestDevnetFaucetInstruction(programID solana.PublicKey, accounts RequestDevnetFaucetAccounts) solana.Instruction {
	return newInstruction(programID, solana.AccountMetaSlice{
		writableSigner(accounts.User),
		writable(accounts.FaucetTracker),
		writable(accounts.USDTMint),
		writable(accounts.UserUSDTATA),
		readonly(accounts.USDTTreasuryPDA),
		readonly(accounts.TokenProgram),
		readonly(accounts.AssociatedTokenProgram),
		readonly(accounts.SystemProgram),
	}, RequestDevnetFaucetDiscriminator[:])
}

func ValidateStatInstruction(programID, dailyScoresMerkleRoots solana.PublicKey, validation ScoresStatValidation, predicate TraderPredicate, op *BinaryExpression) (solana.Instruction, error) {
	statA := validation.PrimaryStatTerm()
	statB, err := validation.SecondaryStatTerm()
	if err != nil {
		return nil, err
	}
	data := append([]byte{}, ValidateStatDiscriminator[:]...)
	data = appendI64(data, validation.Summary.UpdateStats.MinTimestamp)
	data = encodeScoresBatchSummary(data, validation.FixtureSummaryInput())
	data, err = encodeProofVec(data, validation.SubTreeProof)
	if err != nil {
		return nil, err
	}
	data, err = encodeProofVec(data, validation.MainTreeProof)
	if err != nil {
		return nil, err
	}
	data = encodeTraderPredicate(data, predicate)
	data, err = encodeStatTerm(data, statA)
	if err != nil {
		return nil, err
	}
	data, err = encodeStatTermOption(data, statB)
	if err != nil {
		return nil, err
	}
	data = encodeBinaryExpressionOption(data, op)
	return newInstruction(programID, solana.AccountMetaSlice{readonly(dailyScoresMerkleRoots)}, data), nil
}

func DevnetValidateStatInstruction(programID solana.PublicKey, validation ScoresStatValidation, predicate TraderPredicate, op *BinaryExpression) (solana.Instruction, error) {
	day, err := validation.EpochDay()
	if err != nil {
		return nil, err
	}
	root := NewDevnetPDAs().DailyScoresRoots(day).Address
	return ValidateStatInstruction(programID, root, validation, predicate, op)
}

func ValidateStatV2Instruction(programID, dailyScoresMerkleRoots solana.PublicKey, payload StatValidationInput, strategy NDimensionalStrategy) (solana.Instruction, error) {
	if err := strategy.ValidateIndices(len(payload.Stats)); err != nil {
		return nil, err
	}
	data := append([]byte{}, ValidateStatV2Discriminator[:]...)
	var err error
	data, err = encodeStatValidationInput(data, payload)
	if err != nil {
		return nil, err
	}
	data, err = encodeNDimensionalStrategy(data, strategy)
	if err != nil {
		return nil, err
	}
	return newInstruction(programID, solana.AccountMetaSlice{readonly(dailyScoresMerkleRoots)}, data), nil
}

func DevnetValidateStatV2Instruction(programID solana.PublicKey, payload StatValidationInput, strategy NDimensionalStrategy) (solana.Instruction, error) {
	day, err := TimestampMSToEpochDay(payload.Ts)
	if err != nil {
		return nil, err
	}
	root := NewDevnetPDAs().DailyScoresRoots(day).Address
	return ValidateStatV2Instruction(programID, root, payload, strategy)
}

func ValidateFixtureInstruction(programID, tenDailyFixturesRoots solana.PublicKey, validation FixtureValidation) (solana.Instruction, error) {
	data := append([]byte{}, ValidateFixtureDiscriminator[:]...)
	var err error
	data, err = encodeFixture(data, validation.Snapshot)
	if err != nil {
		return nil, err
	}
	data, err = encodeFixtureBatchSummary(data, validation.Summary)
	if err != nil {
		return nil, err
	}
	data, err = encodeProofVec(data, validation.SubTreeProof)
	if err != nil {
		return nil, err
	}
	data, err = encodeProofVec(data, validation.MainTreeProof)
	if err != nil {
		return nil, err
	}
	return newInstruction(programID, solana.AccountMetaSlice{readonly(tenDailyFixturesRoots)}, data), nil
}

func DevnetValidateFixtureInstruction(programID solana.PublicKey, validation FixtureValidation) (solana.Instruction, error) {
	day, err := TimestampMSToEpochDay(validation.Summary.UpdateStats.MinTimestamp)
	if err != nil {
		return nil, err
	}
	root := NewDevnetPDAs().TenDailyFixturesRoots(day).Address
	return ValidateFixtureInstruction(programID, root, validation)
}

func ValidateFixtureBatchInstruction(programID, tenDailyFixturesRoots solana.PublicKey, index uint8, validation FixtureBatchValidation) (solana.Instruction, error) {
	data := append([]byte{}, ValidateFixtureBatchDiscriminator[:]...)
	data = append(data, index)
	data = encodeBatchMetadata(data, validation.Metadata)
	var err error
	data, err = encodeProofVec(data, validation.Proof)
	if err != nil {
		return nil, err
	}
	return newInstruction(programID, solana.AccountMetaSlice{readonly(tenDailyFixturesRoots)}, data), nil
}

func DevnetValidateFixtureBatchInstruction(programID solana.PublicKey, epochDay uint16, index uint8, validation FixtureBatchValidation) (solana.Instruction, error) {
	root := NewDevnetPDAs().TenDailyFixturesRoots(epochDay).Address
	return ValidateFixtureBatchInstruction(programID, root, index, validation)
}

func ValidateOddsInstruction(programID, dailyOddsMerkleRoots solana.PublicKey, validation OddsValidation) (solana.Instruction, error) {
	data := append([]byte{}, ValidateOddsDiscriminator[:]...)
	data = appendI64(data, validation.Odds.Ts)
	var err error
	data, err = encodeOdds(data, validation.Odds)
	if err != nil {
		return nil, err
	}
	data, err = encodeOddsBatchSummary(data, validation.Summary)
	if err != nil {
		return nil, err
	}
	data, err = encodeProofVec(data, validation.SubTreeProof)
	if err != nil {
		return nil, err
	}
	data, err = encodeProofVec(data, validation.MainTreeProof)
	if err != nil {
		return nil, err
	}
	return newInstruction(programID, solana.AccountMetaSlice{readonly(dailyOddsMerkleRoots)}, data), nil
}

func DevnetValidateOddsInstruction(programID solana.PublicKey, validation OddsValidation) (solana.Instruction, error) {
	day, err := TimestampMSToEpochDay(validation.Summary.UpdateStats.MinTimestamp)
	if err != nil {
		return nil, err
	}
	root := NewDevnetPDAs().DailyOddsMerkleRoots(day).Address
	return ValidateOddsInstruction(programID, root, validation)
}

type CreateIntentAccounts struct {
	Maker             solana.PublicKey
	OrderIntent       solana.PublicKey
	IntentVault       solana.PublicKey
	MakerTokenAccount solana.PublicKey
	TokenMint         solana.PublicKey
	TokenTreasuryPDA  solana.PublicKey
	TokenProgram      solana.PublicKey
	SystemProgram     solana.PublicKey
}

type CreateIntentParams struct {
	IntentID      uint64
	TermsHash     [32]byte
	DepositAmount uint64
	ExpirationTS  int64
	ClaimPeriod   uint16
	FixtureID     int64
}

func CreateIntentInstruction(programID solana.PublicKey, accounts CreateIntentAccounts, params CreateIntentParams) (solana.Instruction, error) {
	data := append([]byte{}, CreateIntentDiscriminator[:]...)
	data = appendU64(data, params.IntentID)
	data = append(data, params.TermsHash[:]...)
	data = appendU64(data, params.DepositAmount)
	data = appendI64(data, params.ExpirationTS)
	data = appendU16(data, params.ClaimPeriod)
	data = appendI64(data, params.FixtureID)
	return newInstruction(programID, solana.AccountMetaSlice{
		writableSigner(accounts.Maker), writable(accounts.OrderIntent), writable(accounts.IntentVault), writable(accounts.MakerTokenAccount),
		readonly(accounts.TokenMint), readonly(accounts.TokenTreasuryPDA), readonly(accounts.TokenProgram), readonly(accounts.SystemProgram),
	}, data), nil
}

type CreateTradeAccounts struct {
	Authority           solana.PublicKey
	TraderA             solana.PublicKey
	TraderB             solana.PublicKey
	TraderATokenAccount solana.PublicKey
	TraderBTokenAccount solana.PublicKey
	TradeEscrow         solana.PublicKey
	EscrowVault         solana.PublicKey
	StakeTokenMint      solana.PublicKey
	TokenTreasuryPDA    solana.PublicKey
	TokenProgram        solana.PublicKey
	SystemProgram       solana.PublicKey
}

type CreateTradeParams struct {
	TradeID        uint64
	StakeA         uint64
	StakeB         uint64
	TradeTermsHash [32]byte
}

func CreateTradeInstruction(programID solana.PublicKey, accounts CreateTradeAccounts, params CreateTradeParams) (solana.Instruction, error) {
	data := append([]byte{}, CreateTradeDiscriminator[:]...)
	data = appendU64(data, params.TradeID)
	data = appendU64(data, params.StakeA)
	data = appendU64(data, params.StakeB)
	data = append(data, params.TradeTermsHash[:]...)
	return newInstruction(programID, solana.AccountMetaSlice{
		writableSigner(accounts.Authority), writableSigner(accounts.TraderA), writableSigner(accounts.TraderB),
		writable(accounts.TraderATokenAccount), writable(accounts.TraderBTokenAccount), writable(accounts.TradeEscrow), writable(accounts.EscrowVault),
		readonly(accounts.StakeTokenMint), readonly(accounts.TokenTreasuryPDA), readonly(accounts.TokenProgram), readonly(accounts.SystemProgram),
	}, data), nil
}

type ExecuteMatchAccounts struct {
	Solver        solana.PublicKey
	MakerIntent   solana.PublicKey
	TakerIntent   solana.PublicKey
	MakerVault    solana.PublicKey
	TakerVault    solana.PublicKey
	MatchedTrade  solana.PublicKey
	TradeVault    solana.PublicKey
	TokenMint     solana.PublicKey
	TokenProgram  solana.PublicKey
	SystemProgram solana.PublicKey
}

type ExecuteMatchParams struct {
	TradeID    uint64
	MakerStake uint64
	TakerStake uint64
}

func ExecuteMatchInstruction(programID solana.PublicKey, accounts ExecuteMatchAccounts, params ExecuteMatchParams) (solana.Instruction, error) {
	data := append([]byte{}, ExecuteMatchDiscriminator[:]...)
	data = appendU64(data, params.TradeID)
	data = appendU64(data, params.MakerStake)
	data = appendU64(data, params.TakerStake)
	return newInstruction(programID, solana.AccountMetaSlice{
		writableSigner(accounts.Solver), writable(accounts.MakerIntent), writable(accounts.TakerIntent), writable(accounts.MakerVault), writable(accounts.TakerVault),
		writable(accounts.MatchedTrade), writable(accounts.TradeVault), readonly(accounts.TokenMint), readonly(accounts.TokenProgram), readonly(accounts.SystemProgram),
	}, data), nil
}

type CloseIntentAccounts struct {
	Maker             solana.PublicKey
	Authority         solana.PublicKey
	OrderIntent       solana.PublicKey
	IntentVault       solana.PublicKey
	MakerTokenAccount solana.PublicKey
	TokenMint         solana.PublicKey
	TokenProgram      solana.PublicKey
	TokenTreasuryPDA  solana.PublicKey
}

type CloseIntentParams struct{}

func CloseIntentInstruction(programID solana.PublicKey, accounts CloseIntentAccounts, _ CloseIntentParams) (solana.Instruction, error) {
	return newInstruction(programID, solana.AccountMetaSlice{
		writable(accounts.Maker), writableSigner(accounts.Authority), writable(accounts.OrderIntent), writable(accounts.IntentVault), writable(accounts.MakerTokenAccount),
		readonly(accounts.TokenMint), readonly(accounts.TokenProgram), readonly(accounts.TokenTreasuryPDA),
	}, CloseIntentDiscriminator[:]), nil
}

type SettleTradeAccounts struct {
	Winner                 solana.PublicKey
	DailyScoresMerkleRoots solana.PublicKey
	TradeEscrow            solana.PublicKey
	EscrowVault            solana.PublicKey
	WinnerTokenAccount     solana.PublicKey
	TokenMint              solana.PublicKey
	TokenTreasuryPDA       solana.PublicKey
	TokenProgram           solana.PublicKey
	SystemProgram          solana.PublicKey
}

type SettleTradeParams struct {
	TradeID        uint64
	Ts             int64
	FixtureSummary FixtureSummaryInput
	FixtureProof   []ProofNode
	MainTreeProof  []ProofNode
	Predicate      TraderPredicate
	StatA          StatTermInput
	StatB          *StatTermInput
	Op             *BinaryExpression
}

func SettleTradeInstruction(programID solana.PublicKey, accounts SettleTradeAccounts, params SettleTradeParams) (solana.Instruction, error) {
	data := append([]byte{}, SettleTradeDiscriminator[:]...)
	data = appendU64(data, params.TradeID)
	data = appendI64(data, params.Ts)
	data = encodeScoresBatchSummary(data, params.FixtureSummary)
	var err error
	data, err = encodeProofVec(data, params.FixtureProof)
	if err != nil {
		return nil, err
	}
	data, err = encodeProofVec(data, params.MainTreeProof)
	if err != nil {
		return nil, err
	}
	data = encodeTraderPredicate(data, params.Predicate)
	data, err = encodeStatTerm(data, params.StatA)
	if err != nil {
		return nil, err
	}
	data, err = encodeStatTermOption(data, params.StatB)
	if err != nil {
		return nil, err
	}
	data = encodeBinaryExpressionOption(data, params.Op)
	return newInstruction(programID, solana.AccountMetaSlice{
		writableSigner(accounts.Winner), readonly(accounts.DailyScoresMerkleRoots), writable(accounts.TradeEscrow), writable(accounts.EscrowVault), writable(accounts.WinnerTokenAccount),
		readonly(accounts.TokenMint), readonly(accounts.TokenTreasuryPDA), readonly(accounts.TokenProgram), readonly(accounts.SystemProgram),
	}, data), nil
}

type SettleMatchedTradeAccounts struct {
	Winner                 solana.PublicKey
	DailyScoresMerkleRoots solana.PublicKey
	MatchedTrade           solana.PublicKey
	TradeVault             solana.PublicKey
	WinnerTokenAccount     solana.PublicKey
	TokenMint              solana.PublicKey
	TokenTreasuryPDA       solana.PublicKey
	TokenProgram           solana.PublicKey
	SystemProgram          solana.PublicKey
}

type SettleMatchedTradeParams struct {
	TradeID        uint64
	Ts             int64
	FixtureSummary FixtureSummaryInput
	FixtureProof   []ProofNode
	MainTreeProof  []ProofNode
	StatA          StatTermInput
	StatB          *StatTermInput
	Terms          MarketIntentParams
}

func SettleMatchedTradeInstruction(programID solana.PublicKey, accounts SettleMatchedTradeAccounts, params SettleMatchedTradeParams) (solana.Instruction, error) {
	data := append([]byte{}, SettleMatchedTradeDiscriminator[:]...)
	data = appendU64(data, params.TradeID)
	data = appendI64(data, params.Ts)
	data = encodeScoresBatchSummary(data, params.FixtureSummary)
	var err error
	data, err = encodeProofVec(data, params.FixtureProof)
	if err != nil {
		return nil, err
	}
	data, err = encodeProofVec(data, params.MainTreeProof)
	if err != nil {
		return nil, err
	}
	data, err = encodeStatTerm(data, params.StatA)
	if err != nil {
		return nil, err
	}
	data, err = encodeStatTermOption(data, params.StatB)
	if err != nil {
		return nil, err
	}
	data, err = encodeMarketIntentParams(data, params.Terms)
	if err != nil {
		return nil, err
	}
	return newInstruction(programID, solana.AccountMetaSlice{
		writableSigner(accounts.Winner), readonly(accounts.DailyScoresMerkleRoots), writable(accounts.MatchedTrade), writable(accounts.TradeVault), writable(accounts.WinnerTokenAccount),
		readonly(accounts.TokenMint), readonly(accounts.TokenTreasuryPDA), readonly(accounts.TokenProgram), readonly(accounts.SystemProgram),
	}, data), nil
}

type ClaimViaResolutionAccounts struct {
	Winner               solana.PublicKey
	DailyResolutionRoots solana.PublicKey
	MatchedTrade         solana.PublicKey
	TradeVault           solana.PublicKey
	WinnerTokenAccount   solana.PublicKey
	TokenProgram         solana.PublicKey
}

type ClaimViaResolutionParams struct {
	EpochDay      uint16
	IntervalIndex uint16
	MerkleProof   []ProofNode
}

func ClaimViaResolutionInstruction(programID solana.PublicKey, accounts ClaimViaResolutionAccounts, params ClaimViaResolutionParams) (solana.Instruction, error) {
	data := append([]byte{}, ClaimViaResolutionDiscriminator[:]...)
	data = appendU16(data, params.EpochDay)
	data = appendU16(data, params.IntervalIndex)
	var err error
	data, err = encodeProofVec(data, params.MerkleProof)
	if err != nil {
		return nil, err
	}
	return newInstruction(programID, solana.AccountMetaSlice{
		writableSigner(accounts.Winner), readonly(accounts.DailyResolutionRoots), writable(accounts.MatchedTrade), writable(accounts.TradeVault), writable(accounts.WinnerTokenAccount), readonly(accounts.TokenProgram),
	}, data), nil
}

type ClaimBatchLegacyAccounts struct {
	Payer                solana.PublicKey
	DailyResolutionRoots solana.PublicKey
	TokenMint            solana.PublicKey
	TokenProgram         solana.PublicKey
	SystemProgram        solana.PublicKey
}

type ClaimBatchLegacyParams struct {
	EpochDay      uint16
	IntervalIndex uint16
	TermsHash     [32]byte
	WinnerIsMaker bool
	Seq           uint32
	MerkleProof   []ProofNode
}

func ClaimBatchLegacyInstruction(programID solana.PublicKey, accounts ClaimBatchLegacyAccounts, params ClaimBatchLegacyParams) (solana.Instruction, error) {
	data := append([]byte{}, ClaimBatchLegacyDiscriminator[:]...)
	data = appendU16(data, params.EpochDay)
	data = appendU16(data, params.IntervalIndex)
	data = append(data, params.TermsHash[:]...)
	data = appendBool(data, params.WinnerIsMaker)
	data = appendU32(data, params.Seq)
	var err error
	data, err = encodeProofVec(data, params.MerkleProof)
	if err != nil {
		return nil, err
	}
	return newInstruction(programID, solana.AccountMetaSlice{
		writableSigner(accounts.Payer), readonly(accounts.DailyResolutionRoots), readonly(accounts.TokenMint), readonly(accounts.TokenProgram), readonly(accounts.SystemProgram),
	}, data), nil
}

type RefundBatchAccounts struct {
	Payer         solana.PublicKey
	TokenMint     solana.PublicKey
	TokenProgram  solana.PublicKey
	SystemProgram solana.PublicKey
}

type RefundBatchParams struct{}

func RefundBatchInstruction(programID solana.PublicKey, accounts RefundBatchAccounts, _ RefundBatchParams) (solana.Instruction, error) {
	return newInstruction(programID, solana.AccountMetaSlice{
		writableSigner(accounts.Payer), readonly(accounts.TokenMint), readonly(accounts.TokenProgram), readonly(accounts.SystemProgram),
	}, RefundBatchDiscriminator[:]), nil
}

type AuditTradeResultAccounts struct {
	Payer                  solana.PublicKey
	DailyScoresMerkleRoots solana.PublicKey
}

type AuditTradeResultParams struct {
	Terms          MarketIntentParams
	FixtureSummary FixtureSummaryInput
	MainTreeProof  []ProofNode
	FixtureProof   []ProofNode
	StatA          StatTermInput
	StatB          *StatTermInput
	Ts             int64
}

func AuditTradeResultInstruction(programID solana.PublicKey, accounts AuditTradeResultAccounts, params AuditTradeResultParams) (solana.Instruction, error) {
	data := append([]byte{}, AuditTradeResultDiscriminator[:]...)
	var err error
	data, err = encodeMarketIntentParams(data, params.Terms)
	if err != nil {
		return nil, err
	}
	data = encodeScoresBatchSummary(data, params.FixtureSummary)
	data, err = encodeProofVec(data, params.MainTreeProof)
	if err != nil {
		return nil, err
	}
	data, err = encodeProofVec(data, params.FixtureProof)
	if err != nil {
		return nil, err
	}
	data, err = encodeStatTerm(data, params.StatA)
	if err != nil {
		return nil, err
	}
	data, err = encodeStatTermOption(data, params.StatB)
	if err != nil {
		return nil, err
	}
	data = appendI64(data, params.Ts)
	return newInstruction(programID, solana.AccountMetaSlice{writableSigner(accounts.Payer), readonly(accounts.DailyScoresMerkleRoots)}, data), nil
}

type MarketIntentParams struct {
	FixtureID int64
	Period    uint16
	StatAKey  uint32
	StatBKey  *uint32
	Predicate TraderPredicate
	Op        *BinaryExpression
	Negation  bool
}

func encodeMarketIntentParams(out []byte, terms MarketIntentParams) ([]byte, error) {
	out = appendI64(out, terms.FixtureID)
	out = appendU16(out, terms.Period)
	out = appendU32(out, terms.StatAKey)
	if terms.StatBKey == nil {
		out = append(out, 0)
	} else {
		out = append(out, 1)
		out = appendU32(out, *terms.StatBKey)
	}
	out = encodeTraderPredicate(out, terms.Predicate)
	out = encodeBinaryExpressionOption(out, terms.Op)
	out = appendBool(out, terms.Negation)
	return out, nil
}

func encodeStatValidationInput(out []byte, input StatValidationInput) ([]byte, error) {
	out = appendI64(out, input.Ts)
	out = encodeScoresBatchSummary(out, input.FixtureSummary)
	var err error
	out, err = encodeProofVec(out, input.FixtureProof)
	if err != nil {
		return nil, err
	}
	out, err = encodeProofVec(out, input.MainTreeProof)
	if err != nil {
		return nil, err
	}
	out = append(out, input.EventStatRoot[:]...)
	out, err = putLen(out, len(input.Stats))
	if err != nil {
		return nil, err
	}
	for _, stat := range input.Stats {
		out = encodeScoreStat(out, stat.Stat)
		out, err = encodeProofVec(out, stat.StatProof)
		if err != nil {
			return nil, err
		}
	}
	return out, nil
}

func encodeNDimensionalStrategy(out []byte, strategy NDimensionalStrategy) ([]byte, error) {
	var err error
	out, err = putLen(out, len(strategy.GeometricTargets))
	if err != nil {
		return nil, err
	}
	for _, target := range strategy.GeometricTargets {
		out = append(out, target.StatIndex)
		out = appendI32(out, target.Prediction)
	}
	if strategy.DistancePredicate == nil {
		out = append(out, 0)
	} else {
		out = append(out, 1)
		out = encodeTraderPredicate(out, *strategy.DistancePredicate)
	}
	out, err = putLen(out, len(strategy.DiscretePredicates))
	if err != nil {
		return nil, err
	}
	for _, predicate := range strategy.DiscretePredicates {
		switch predicate.Kind {
		case StatPredicateSingle:
			out = append(out, 0, predicate.Index)
			out = encodeTraderPredicate(out, predicate.Predicate)
		case StatPredicateBinary:
			out = append(out, 1, predicate.IndexA, predicate.IndexB)
			out = append(out, byte(predicate.Op))
			out = encodeTraderPredicate(out, predicate.Predicate)
		default:
			return nil, &Error{Kind: ErrValidation, Msg: fmt.Sprintf("unknown stat predicate kind %d", predicate.Kind)}
		}
	}
	return out, nil
}

func encodeFixture(out []byte, fixture Fixture) ([]byte, error) {
	out = appendI64(out, fixture.Ts)
	out = appendI64(out, fixture.StartTime)
	var err error
	out, err = putString(out, fixture.Competition)
	if err != nil {
		return nil, err
	}
	out = appendI32(out, fixture.CompetitionID)
	out = appendI32(out, fixture.FixtureGroupID)
	out = appendI32(out, fixture.Participant1ID)
	out, err = putString(out, fixture.Participant1)
	if err != nil {
		return nil, err
	}
	out = appendI32(out, fixture.Participant2ID)
	out, err = putString(out, fixture.Participant2)
	if err != nil {
		return nil, err
	}
	out = appendI64(out, fixture.FixtureID)
	out = appendBool(out, fixture.Participant1IsHome)
	return out, nil
}

func encodeFixtureBatchSummary(out []byte, summary FixtureBatchSummary) ([]byte, error) {
	out = appendI64(out, summary.FixtureID)
	out = appendI32(out, summary.CompetitionID)
	var err error
	out, err = putString(out, summary.Competition)
	if err != nil {
		return nil, err
	}
	out, err = encodeUpdateStatsU32(out, summary.UpdateStats)
	if err != nil {
		return nil, err
	}
	out = append(out, summary.UpdateSubTreeRoot[:]...)
	return out, nil
}

func encodeBatchMetadata(out []byte, metadata BatchMetadata) []byte {
	out = appendI32(out, metadata.TotalUpdateCount)
	out = appendI32(out, metadata.NumUniqueFixtures)
	out = appendI64(out, metadata.OverallBatchStartTs)
	out = appendI64(out, metadata.OverallBatchEndTs)
	return out
}

func encodeOdds(out []byte, odds OddsPayload) ([]byte, error) {
	out = appendI64(out, odds.FixtureID)
	var err error
	out, err = putString(out, odds.MessageID)
	if err != nil {
		return nil, err
	}
	out = appendI64(out, odds.Ts)
	out, err = putString(out, odds.Bookmaker)
	if err != nil {
		return nil, err
	}
	out = appendI32(out, odds.BookmakerID)
	out, err = putString(out, odds.SuperOddsType)
	if err != nil {
		return nil, err
	}
	out, err = encodeStringOption(out, odds.GameState)
	if err != nil {
		return nil, err
	}
	out = appendBool(out, odds.InRunning)
	out, err = encodeStringOption(out, odds.MarketParameters)
	if err != nil {
		return nil, err
	}
	out, err = encodeStringOption(out, odds.MarketPeriod)
	if err != nil {
		return nil, err
	}
	out, err = putLen(out, len(odds.PriceNames))
	if err != nil {
		return nil, err
	}
	for _, priceName := range odds.PriceNames {
		out, err = putString(out, priceName)
		if err != nil {
			return nil, err
		}
	}
	out, err = putLen(out, len(odds.Prices))
	if err != nil {
		return nil, err
	}
	for _, price := range odds.Prices {
		out = appendI32(out, price)
	}
	return out, nil
}

func encodeOddsBatchSummary(out []byte, summary OddsBatchSummary) ([]byte, error) {
	out = appendI64(out, summary.FixtureID)
	var err error
	out, err = encodeUpdateStatsU32(out, summary.UpdateStats)
	if err != nil {
		return nil, err
	}
	out = append(out, summary.OddsSubTreeRoot[:]...)
	return out, nil
}

func encodeUpdateStatsU32(out []byte, stats UpdateStats) ([]byte, error) {
	count, err := nonnegativeU32(stats.UpdateCount, "update_count")
	if err != nil {
		return nil, err
	}
	out = appendU32(out, count)
	out = appendI64(out, stats.MinTimestamp)
	out = appendI64(out, stats.MaxTimestamp)
	return out, nil
}

func encodeScoresBatchSummary(out []byte, summary FixtureSummaryInput) []byte {
	out = appendI64(out, summary.FixtureID)
	out = appendI32(out, summary.UpdateCount)
	out = appendI64(out, summary.MinTimestamp)
	out = appendI64(out, summary.MaxTimestamp)
	out = append(out, summary.EventsSubTreeRoot[:]...)
	return out
}

func encodeStatTerm(out []byte, term StatTermInput) ([]byte, error) {
	out = encodeScoreStat(out, term.StatToProve)
	out = append(out, term.EventStatRoot[:]...)
	return encodeProofVec(out, term.StatProof)
}

func encodeStatTermOption(out []byte, term *StatTermInput) ([]byte, error) {
	if term == nil {
		return append(out, 0), nil
	}
	out = append(out, 1)
	return encodeStatTerm(out, *term)
}

func encodeScoreStat(out []byte, stat ScoreStat) []byte {
	out = appendU32(out, stat.Key)
	out = appendI32(out, stat.Value)
	out = appendI32(out, stat.Period)
	return out
}

func encodeProofVec(out []byte, proof []ProofNode) ([]byte, error) {
	out, err := putLen(out, len(proof))
	if err != nil {
		return nil, err
	}
	for _, node := range proof {
		out = append(out, node.Hash[:]...)
		out = appendBool(out, node.IsRightSibling)
	}
	return out, nil
}

func encodeTraderPredicate(out []byte, predicate TraderPredicate) []byte {
	out = appendI32(out, predicate.Threshold)
	out = append(out, byte(predicate.Comparison))
	return out
}

func encodeBinaryExpressionOption(out []byte, op *BinaryExpression) []byte {
	if op == nil {
		return append(out, 0)
	}
	return append(out, 1, byte(*op))
}

func encodeStringOption(out []byte, value *string) ([]byte, error) {
	if value == nil {
		return append(out, 0), nil
	}
	out = append(out, 1)
	return putString(out, *value)
}

func nonnegativeU32(value int32, name string) (uint32, error) {
	if value < 0 {
		return 0, &Error{Kind: ErrValidation, Msg: fmt.Sprintf("%s must be nonnegative to match the Devnet IDL u32 field", name)}
	}
	return uint32(value), nil
}

func putString(out []byte, value string) ([]byte, error) {
	if len(value) > math.MaxUint32 {
		return nil, &Error{Kind: ErrValidation, Msg: "Anchor string length exceeds u32"}
	}
	out = appendU32(out, uint32(len(value)))
	out = append(out, value...)
	return out, nil
}

func putLen(out []byte, length int) ([]byte, error) {
	if length < 0 || length > math.MaxUint32 {
		return nil, &Error{Kind: ErrValidation, Msg: "Anchor vector length exceeds u32"}
	}
	return appendU32(out, uint32(length)), nil
}

func appendBool(out []byte, value bool) []byte {
	if value {
		return append(out, 1)
	}
	return append(out, 0)
}

func appendU16(out []byte, value uint16) []byte {
	return binary.LittleEndian.AppendUint16(out, value)
}

func appendU32(out []byte, value uint32) []byte {
	return binary.LittleEndian.AppendUint32(out, value)
}

func appendU64(out []byte, value uint64) []byte {
	return binary.LittleEndian.AppendUint64(out, value)
}

func appendI32(out []byte, value int32) []byte {
	return appendU32(out, uint32(value))
}

func appendI64(out []byte, value int64) []byte {
	return appendU64(out, uint64(value))
}
