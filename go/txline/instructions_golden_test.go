package txline

import (
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/gagliardetto/solana-go"
)

func TestValidationInstructionBytesMatchRustGoldenFixtures(t *testing.T) {
	programID := keyForTest(200)
	root := keyForTest(201)

	op := Add()
	statIX, err := ValidateStatInstruction(programID, root, scoreValidationGolden(), NewTraderPredicate(1, LessThan()), &op)
	if err != nil {
		t.Fatal(err)
	}
	assertInstructionData(t, statIX, validationGolden(t, "validate_stat"))

	statV2IX, err := ValidateStatV2Instruction(programID, root, statV2PayloadGolden(), v2StrategyGolden())
	if err != nil {
		t.Fatal(err)
	}
	assertInstructionData(t, statV2IX, validationGolden(t, "validate_stat_v2"))

	fixtureIX, err := ValidateFixtureInstruction(programID, root, fixtureValidationGolden())
	if err != nil {
		t.Fatal(err)
	}
	assertInstructionData(t, fixtureIX, validationGolden(t, "validate_fixture"))

	fixtureBatchIX, err := ValidateFixtureBatchInstruction(programID, root, 3, fixtureBatchValidationGolden())
	if err != nil {
		t.Fatal(err)
	}
	assertInstructionData(t, fixtureBatchIX, validationGolden(t, "validate_fixture_batch"))

	oddsIX, err := ValidateOddsInstruction(programID, root, oddsValidationGolden())
	if err != nil {
		t.Fatal(err)
	}
	assertInstructionData(t, oddsIX, validationGolden(t, "validate_odds"))
}

func TestTradingInstructionBytesMatchRustGoldenFixtures(t *testing.T) {
	programID := keyForTest(200)
	assertDataFromBuilder(t, "create_intent", func() (solana.Instruction, error) {
		return CreateIntentInstruction(programID, createIntentAccountsGolden(), createIntentParamsGolden())
	})
	assertDataFromBuilder(t, "create_trade", func() (solana.Instruction, error) {
		return CreateTradeInstruction(programID, createTradeAccountsGolden(), createTradeParamsGolden())
	})
	assertDataFromBuilder(t, "execute_match", func() (solana.Instruction, error) {
		return ExecuteMatchInstruction(programID, executeMatchAccountsGolden(), executeMatchParamsGolden())
	})
	assertDataFromBuilder(t, "close_intent", func() (solana.Instruction, error) {
		return CloseIntentInstruction(programID, closeIntentAccountsGolden(), CloseIntentParams{})
	})
	assertDataFromBuilder(t, "settle_trade", func() (solana.Instruction, error) {
		return SettleTradeInstruction(programID, settleTradeAccountsGolden(), settleTradeParamsGolden())
	})
	assertDataFromBuilder(t, "settle_matched_trade", func() (solana.Instruction, error) {
		return SettleMatchedTradeInstruction(programID, settleMatchedTradeAccountsGolden(), settleMatchedTradeParamsGolden())
	})
	assertDataFromBuilder(t, "claim_via_resolution", func() (solana.Instruction, error) {
		return ClaimViaResolutionInstruction(programID, claimViaResolutionAccountsGolden(), claimViaResolutionParamsGolden())
	})
	assertDataFromBuilder(t, "claim_batch_legacy", func() (solana.Instruction, error) {
		return ClaimBatchLegacyInstruction(programID, claimBatchLegacyAccountsGolden(), claimBatchLegacyParamsGolden())
	})
	assertDataFromBuilder(t, "refund_batch", func() (solana.Instruction, error) {
		return RefundBatchInstruction(programID, refundBatchAccountsGolden(), RefundBatchParams{})
	})
	assertDataFromBuilder(t, "audit_trade_result", func() (solana.Instruction, error) {
		return AuditTradeResultInstruction(programID, auditTradeResultAccountsGolden(), auditTradeResultParamsGolden())
	})
}

func TestFixtureAndOddsValidationRejectNegativeUpdateCount(t *testing.T) {
	fixture := fixtureValidationGolden()
	fixture.Summary.UpdateStats.UpdateCount = -1
	if _, err := ValidateFixtureInstruction(keyForTest(200), keyForTest(201), fixture); err == nil {
		t.Fatal("negative fixture updateCount should be rejected")
	}
	odds := oddsValidationGolden()
	odds.Summary.UpdateStats.UpdateCount = -1
	if _, err := ValidateOddsInstruction(keyForTest(200), keyForTest(201), odds); err == nil {
		t.Fatal("negative odds updateCount should be rejected")
	}
}

func assertDataFromBuilder(t *testing.T, name string, build func() (solana.Instruction, error)) {
	t.Helper()
	ix, err := build()
	if err != nil {
		t.Fatal(err)
	}
	assertInstructionData(t, ix, tradingGolden(t, name))
}

func assertInstructionData(t *testing.T, ix solana.Instruction, want []byte) {
	t.Helper()
	got, err := ix.Data()
	if err != nil {
		t.Fatal(err)
	}
	if string(got) != string(want) {
		t.Fatalf("instruction data mismatch\ngot  %x\nwant %x", got, want)
	}
}

func scoreValidationGolden() ScoresStatValidation {
	return ScoresStatValidation{
		Ts:            1_781_123_456_789,
		StatToProve:   ScoreStat{Key: 1001, Value: 2, Period: 0},
		EventStatRoot: hash32(20),
		Summary: ScoresBatchSummary{
			FixtureID: i64Max32Plus(6),
			UpdateStats: UpdateStats{
				UpdateCount:  -3,
				MinTimestamp: 1_781_123_456_789,
				MaxTimestamp: 1_781_123_456_799,
			},
			EventStatsSubTreeRoot: hash32(10),
		},
		StatProof:     []ProofNode{proof(30, true)},
		SubTreeProof:  []ProofNode{proof(50, false)},
		MainTreeProof: []ProofNode{proof(60, true)},
		StatToProve2:  &ScoreStat{Key: 1002, Value: -1, Period: 1},
		StatProof2:    []ProofNode{proof(40, false)},
	}
}

func statV2PayloadGolden() StatValidationInput {
	return StatValidationInput{
		Ts: 1_781_123_456_789,
		FixtureSummary: FixtureSummaryInput{
			FixtureID:         i64Max32Plus(6),
			UpdateCount:       -3,
			MinTimestamp:      1_781_123_456_789,
			MaxTimestamp:      1_781_123_456_799,
			EventsSubTreeRoot: hashBytes(10),
		},
		FixtureProof:  []ProofNode{proof(51, false)},
		MainTreeProof: []ProofNode{proof(61, true)},
		EventStatRoot: hashBytes(22),
		Stats: []StatLeafInput{
			{Stat: ScoreStat{Key: 1001, Value: 2, Period: 0}, StatProof: []ProofNode{proof(31, true)}},
			{Stat: ScoreStat{Key: 1002, Value: -1, Period: 1}, StatProof: []ProofNode{proof(41, false)}},
		},
	}
}

func v2StrategyGolden() NDimensionalStrategy {
	p0 := NewTraderPredicate(2, LessThan())
	return NDimensionalStrategy{
		GeometricTargets: []GeometricTarget{
			{StatIndex: 0, Prediction: 0},
			{StatIndex: 1, Prediction: 1},
		},
		DistancePredicate: &p0,
		DiscretePredicates: []StatPredicate{
			{Kind: StatPredicateSingle, Index: 0, Predicate: NewTraderPredicate(1, EqualTo())},
			{Kind: StatPredicateBinary, IndexA: 0, IndexB: 1, Op: Subtract(), Predicate: NewTraderPredicate(0, GreaterThan())},
		},
	}
}

func fixtureValidationGolden() FixtureValidation {
	return FixtureValidation{
		Snapshot: Fixture{
			Ts:                 1_781_123_000_000,
			StartTime:          1_781_126_600_000,
			Competition:        "Devnet Cup",
			CompetitionID:      7,
			FixtureGroupID:     -8,
			Participant1ID:     101,
			Participant1:       "Alpha",
			Participant2ID:     202,
			Participant2:       "Beta",
			FixtureID:          i64Max32Plus(7),
			Participant1IsHome: true,
		},
		Summary: FixtureBatchSummary{
			FixtureID:     i64Max32Plus(7),
			CompetitionID: 7,
			Competition:   "Devnet Cup",
			UpdateStats: UpdateStats{
				UpdateCount:  4,
				MinTimestamp: 1_781_123_000_000,
				MaxTimestamp: 1_781_123_000_001,
			},
			UpdateSubTreeRoot: hash32(70),
		},
		SubTreeProof:  []ProofNode{proof(71, false)},
		MainTreeProof: []ProofNode{proof(72, true)},
	}
}

func fixtureBatchValidationGolden() FixtureBatchValidation {
	return FixtureBatchValidation{
		Metadata: BatchMetadata{
			TotalUpdateCount:    5,
			NumUniqueFixtures:   2,
			OverallBatchStartTs: 1_781_123_000_000,
			OverallBatchEndTs:   1_781_123_900_000,
		},
		Proof: []ProofNode{proof(80, false), proof(81, true)},
	}
}

func oddsValidationGolden() OddsValidation {
	return OddsValidation{
		Odds: OddsPayload{
			FixtureID:        i64Max32Plus(8),
			MessageID:        "msg-1",
			Ts:               1_781_123_456_789,
			Bookmaker:        "Book",
			BookmakerID:      9,
			SuperOddsType:    "Winner",
			GameState:        ptrString("PreMatch"),
			InRunning:        false,
			MarketParameters: nil,
			MarketPeriod:     ptrString("FT"),
			PriceNames:       []string{"Home", "Away"},
			Prices:           []int32{120, -125},
		},
		Summary: OddsBatchSummary{
			FixtureID: i64Max32Plus(8),
			UpdateStats: UpdateStats{
				UpdateCount:  5,
				MinTimestamp: 1_781_123_450_000,
				MaxTimestamp: 1_781_123_459_999,
			},
			OddsSubTreeRoot: hash32(90),
		},
		SubTreeProof:  []ProofNode{proof(91, false)},
		MainTreeProof: []ProofNode{proof(92, true)},
	}
}

func createIntentAccountsGolden() CreateIntentAccounts {
	return CreateIntentAccounts{Maker: keyForTest(1), OrderIntent: keyForTest(2), IntentVault: keyForTest(3), MakerTokenAccount: keyForTest(4), TokenMint: keyForTest(5), TokenTreasuryPDA: keyForTest(6), TokenProgram: keyForTest(7), SystemProgram: keyForTest(8)}
}

func createIntentParamsGolden() CreateIntentParams {
	return CreateIntentParams{IntentID: 9001, TermsHash: hashBytes(100), DepositAmount: 123_456_789, ExpirationTS: 1_781_129_999_999, ClaimPeriod: 42, FixtureID: i64Max32Plus(6)}
}

func createTradeAccountsGolden() CreateTradeAccounts {
	return CreateTradeAccounts{Authority: keyForTest(11), TraderA: keyForTest(12), TraderB: keyForTest(13), TraderATokenAccount: keyForTest(14), TraderBTokenAccount: keyForTest(15), TradeEscrow: keyForTest(16), EscrowVault: keyForTest(17), StakeTokenMint: keyForTest(18), TokenTreasuryPDA: keyForTest(19), TokenProgram: keyForTest(20), SystemProgram: keyForTest(21)}
}

func createTradeParamsGolden() CreateTradeParams {
	return CreateTradeParams{TradeID: 9002, StakeA: 111_111, StakeB: 222_222, TradeTermsHash: hashBytes(110)}
}

func executeMatchAccountsGolden() ExecuteMatchAccounts {
	return ExecuteMatchAccounts{Solver: keyForTest(31), MakerIntent: keyForTest(32), TakerIntent: keyForTest(33), MakerVault: keyForTest(34), TakerVault: keyForTest(35), MatchedTrade: keyForTest(36), TradeVault: keyForTest(37), TokenMint: keyForTest(38), TokenProgram: keyForTest(39), SystemProgram: keyForTest(40)}
}

func executeMatchParamsGolden() ExecuteMatchParams {
	return ExecuteMatchParams{TradeID: 9003, MakerStake: 333_333, TakerStake: 444_444}
}

func closeIntentAccountsGolden() CloseIntentAccounts {
	return CloseIntentAccounts{Maker: keyForTest(51), Authority: keyForTest(52), OrderIntent: keyForTest(53), IntentVault: keyForTest(54), MakerTokenAccount: keyForTest(55), TokenMint: keyForTest(56), TokenProgram: keyForTest(57), TokenTreasuryPDA: keyForTest(58)}
}

func settleTradeAccountsGolden() SettleTradeAccounts {
	return SettleTradeAccounts{Winner: keyForTest(71), DailyScoresMerkleRoots: keyForTest(72), TradeEscrow: keyForTest(73), EscrowVault: keyForTest(74), WinnerTokenAccount: keyForTest(75), TokenMint: keyForTest(76), TokenTreasuryPDA: keyForTest(77), TokenProgram: keyForTest(78), SystemProgram: keyForTest(79)}
}

func settleTradeParamsGolden() SettleTradeParams {
	op := Add()
	return SettleTradeParams{TradeID: 9004, Ts: 1_781_123_456_789, FixtureSummary: fixtureSummaryGolden(), FixtureProof: []ProofNode{proof(50, false)}, MainTreeProof: []ProofNode{proof(60, true)}, Predicate: NewTraderPredicate(1, LessThan()), StatA: statAGolden(), StatB: ptrStatTerm(statBGolden()), Op: &op}
}

func settleMatchedTradeAccountsGolden() SettleMatchedTradeAccounts {
	return SettleMatchedTradeAccounts{Winner: keyForTest(91), DailyScoresMerkleRoots: keyForTest(92), MatchedTrade: keyForTest(93), TradeVault: keyForTest(94), WinnerTokenAccount: keyForTest(95), TokenMint: keyForTest(96), TokenTreasuryPDA: keyForTest(97), TokenProgram: keyForTest(98), SystemProgram: keyForTest(99)}
}

func settleMatchedTradeParamsGolden() SettleMatchedTradeParams {
	return SettleMatchedTradeParams{TradeID: 9005, Ts: 1_781_123_456_790, FixtureSummary: fixtureSummaryGolden(), FixtureProof: []ProofNode{proof(51, false)}, MainTreeProof: []ProofNode{proof(61, true)}, StatA: statAGolden(), StatB: ptrStatTerm(statBGolden()), Terms: marketTermsGolden()}
}

func claimViaResolutionAccountsGolden() ClaimViaResolutionAccounts {
	return ClaimViaResolutionAccounts{Winner: keyForTest(111), DailyResolutionRoots: keyForTest(112), MatchedTrade: keyForTest(113), TradeVault: keyForTest(114), WinnerTokenAccount: keyForTest(115), TokenProgram: keyForTest(116)}
}

func claimViaResolutionParamsGolden() ClaimViaResolutionParams {
	return ClaimViaResolutionParams{EpochDay: 20_615, IntervalIndex: 17, MerkleProof: []ProofNode{proof(70, false), proof(71, true)}}
}

func claimBatchLegacyAccountsGolden() ClaimBatchLegacyAccounts {
	return ClaimBatchLegacyAccounts{Payer: keyForTest(121), DailyResolutionRoots: keyForTest(122), TokenMint: keyForTest(123), TokenProgram: keyForTest(124), SystemProgram: keyForTest(125)}
}

func claimBatchLegacyParamsGolden() ClaimBatchLegacyParams {
	return ClaimBatchLegacyParams{EpochDay: 20_616, IntervalIndex: 18, TermsHash: hashBytes(120), WinnerIsMaker: true, Seq: 941, MerkleProof: []ProofNode{proof(72, false), proof(73, true)}}
}

func refundBatchAccountsGolden() RefundBatchAccounts {
	return RefundBatchAccounts{Payer: keyForTest(131), TokenMint: keyForTest(132), TokenProgram: keyForTest(133), SystemProgram: keyForTest(134)}
}

func auditTradeResultAccountsGolden() AuditTradeResultAccounts {
	return AuditTradeResultAccounts{Payer: keyForTest(141), DailyScoresMerkleRoots: keyForTest(142)}
}

func auditTradeResultParamsGolden() AuditTradeResultParams {
	terms := marketTermsGolden()
	terms.StatBKey = nil
	terms.Op = nil
	terms.Negation = true
	return AuditTradeResultParams{Terms: terms, FixtureSummary: fixtureSummaryGolden(), MainTreeProof: []ProofNode{proof(62, true)}, FixtureProof: []ProofNode{proof(52, false)}, StatA: statAGolden(), Ts: 1_781_123_456_791}
}

func marketTermsGolden() MarketIntentParams {
	op := Subtract()
	statBKey := uint32(1002)
	return MarketIntentParams{FixtureID: i64Max32Plus(6), Period: 0, StatAKey: 1001, StatBKey: &statBKey, Predicate: NewTraderPredicate(1, GreaterThan()), Op: &op, Negation: false}
}

func fixtureSummaryGolden() FixtureSummaryInput {
	return FixtureSummaryInput{FixtureID: i64Max32Plus(6), UpdateCount: -3, MinTimestamp: 1_781_123_456_789, MaxTimestamp: 1_781_123_456_799, EventsSubTreeRoot: hashBytes(10)}
}

func statAGolden() StatTermInput {
	return StatTermInput{StatToProve: ScoreStat{Key: 1001, Value: 2, Period: 0}, EventStatRoot: hashBytes(20), StatProof: []ProofNode{proof(30, true)}}
}

func statBGolden() StatTermInput {
	return StatTermInput{StatToProve: ScoreStat{Key: 1002, Value: -1, Period: 1}, EventStatRoot: hashBytes(20), StatProof: []ProofNode{proof(40, false)}}
}

func ptrStatTerm(value StatTermInput) *StatTermInput { return &value }

func i64Max32Plus(delta int64) int64 { return int64(1<<31-1) + delta }

func keyForTest(base byte) solana.PublicKey {
	var key solana.PublicKey
	for i := range key {
		key[i] = base
	}
	return key
}

func validationGolden(t *testing.T, name string) []byte {
	t.Helper()
	return goldenData(t, filepath.Join("..", "..", "crates", "txline", "tests", "fixtures", "validation_golden.devnet.json"), name)
}

func tradingGolden(t *testing.T, name string) []byte {
	t.Helper()
	return goldenData(t, filepath.Join("..", "..", "crates", "txline", "tests", "fixtures", "trading_golden.devnet.json"), name)
}

func goldenData(t *testing.T, path, name string) []byte {
	t.Helper()
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Fixtures []struct {
			Name    string `json:"name"`
			DataHex string `json:"dataHex"`
		} `json:"fixtures"`
	}
	if err := json.Unmarshal(raw, &file); err != nil {
		t.Fatal(err)
	}
	for _, fixture := range file.Fixtures {
		if fixture.Name == name {
			data, err := hex.DecodeString(fixture.DataHex)
			if err != nil {
				t.Fatal(err)
			}
			return data
		}
	}
	t.Fatalf("missing golden fixture %s", name)
	return nil
}
