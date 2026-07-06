package txline

import (
	"errors"
	"reflect"
	"testing"

	"github.com/gagliardetto/solana-go"
)

func TestFinalOutcomeDetectionRequiresDocumentedShape(t *testing.T) {
	score := finalScoreForTest(2, 1)
	if !IsFinalOutcomeRecord(score) {
		t.Fatal("expected documented final outcome record to be detected")
	}

	score.Action = "score_update"
	if IsFinalOutcomeRecord(score) {
		t.Fatal("non-final action should not be final outcome")
	}

	score = finalScoreForTest(2, 1)
	score.StatusID = ptrInt32(99)
	if IsFinalOutcomeRecord(score) {
		t.Fatal("non-final status should not be final outcome")
	}

	score = finalScoreForTest(2, 1)
	score.Period = ptrInt32(1)
	if IsFinalOutcomeRecord(score) {
		t.Fatal("non-final period should not be final outcome")
	}
}

func TestExtractFinalOutcomeSides(t *testing.T) {
	tests := []struct {
		name string
		p1   int32
		p2   int32
		want MarketSide
	}{
		{name: "participant1", p1: 3, p2: 1, want: MarketSideParticipant1},
		{name: "participant2", p1: 0, p2: 2, want: MarketSideParticipant2},
		{name: "draw", p1: 1, p2: 1, want: MarketSideDraw},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			outcome, err := ExtractFinalOutcome(finalScoreForTest(tt.p1, tt.p2), DefaultSoccerFinalOutcomeConfig())
			if err != nil {
				t.Fatal(err)
			}
			if outcome.Winner != tt.want {
				t.Fatalf("winner mismatch: got %s want %s", outcome.Winner, tt.want)
			}
			if outcome.Participant1Goals != tt.p1 || outcome.Participant2Goals != tt.p2 {
				t.Fatalf("score mismatch: %+v", outcome)
			}
		})
	}
}

func TestExtractFinalOutcomeErrors(t *testing.T) {
	score := finalScoreForTest(1, 0)
	score.Action = "score_update"
	if _, err := ExtractFinalOutcome(score, DefaultSoccerFinalOutcomeConfig()); !errors.Is(err, ErrValidation) {
		t.Fatalf("non-final score should be validation error: %v", err)
	}

	score = finalScoreForTest(1, 0)
	delete(score.Stats, "2")
	if _, err := ExtractFinalOutcome(score, DefaultSoccerFinalOutcomeConfig()); !errors.Is(err, ErrValidation) {
		t.Fatalf("missing stat should be validation error: %v", err)
	}
}

func TestMarketTermsMappingAndHashPropagation(t *testing.T) {
	terms, err := FinalOutcomeMarketTerms(17_952_170, MarketSideParticipant2, DefaultSoccerFinalOutcomeConfig())
	if err != nil {
		t.Fatal(err)
	}
	got, err := terms.MarketIntentParams()
	if err != nil {
		t.Fatal(err)
	}
	again, err := terms.MarketIntentParams()
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(got, again) {
		t.Fatalf("market term mapping is not deterministic: %+v vs %+v", got, again)
	}
	if got.FixtureID != 17_952_170 || got.Period != uint16(FinalSettlementPeriod) || got.StatAKey != 2 || got.StatBKey == nil || *got.StatBKey != 1 {
		t.Fatalf("unexpected final outcome market mapping: %+v", got)
	}
	if got.Op == nil || *got.Op != Subtract() || got.Predicate.Comparison != GreaterThan() || got.Predicate.Threshold != 0 {
		t.Fatalf("unexpected final outcome predicate mapping: %+v", got)
	}

	hash := hashBytes(42)
	planParams := CreateIntentPlanParams{
		Accounts:      createIntentAccountsGolden(),
		Terms:         terms,
		TermsHash:     hash,
		IntentID:      99,
		DepositAmount: 123,
		ExpirationTS:  1_800_000_000_000,
		ClaimPeriod:   7,
	}
	low, err := planParams.CreateIntentParams()
	if err != nil {
		t.Fatal(err)
	}
	if low.TermsHash != hash {
		t.Fatal("explicit terms hash changed while building low-level params")
	}
	plan, err := CreateIntentPlan(planParams)
	if err != nil {
		t.Fatal(err)
	}
	data, err := plan.Instructions[0].Data()
	if err != nil {
		t.Fatal(err)
	}
	var gotHash [32]byte
	copy(gotHash[:], data[16:48])
	if gotHash != hash {
		t.Fatalf("instruction terms hash mismatch: got %x want %x", gotHash, hash)
	}
}

func TestAdditionalScoreMarketTermHelpers(t *testing.T) {
	total, err := TotalGoalsMarketTerms(17_952_170, 2, GreaterThan(), DefaultSoccerFinalOutcomeConfig())
	if err != nil {
		t.Fatal(err)
	}
	if total.Kind != ScoreMarketTotalGoals || total.Op == nil || *total.Op != Add() {
		t.Fatalf("unexpected total goals terms: %+v", total)
	}

	spread, err := SpreadMarketTerms(17_952_170, MarketSideParticipant1, -1, GreaterThan(), DefaultSoccerFinalOutcomeConfig())
	if err != nil {
		t.Fatal(err)
	}
	if spread.Kind != ScoreMarketSpread || spread.Op == nil || *spread.Op != Subtract() {
		t.Fatalf("unexpected spread terms: %+v", spread)
	}
}

func TestFinalOutcomeStrategyShapes(t *testing.T) {
	assertFinalOutcomeStrategy(t, MarketSideParticipant1, 0, 1, GreaterThan())
	assertFinalOutcomeStrategy(t, MarketSideParticipant2, 1, 0, GreaterThan())
	assertFinalOutcomeStrategy(t, MarketSideDraw, 0, 1, EqualTo())
}

func TestFinalOutcomeProofPreservesStatOrderAndBuildsInstruction(t *testing.T) {
	outcome, err := ExtractFinalOutcome(finalScoreForTest(2, 1), DefaultSoccerFinalOutcomeConfig())
	if err != nil {
		t.Fatal(err)
	}
	validation, err := NewScoresStatValidationV2(
		FinalOutcomeStatKeys(outcome.Config),
		finalOutcomeV2ResponseForTest(outcome),
	)
	if err != nil {
		t.Fatal(err)
	}
	proof, err := NewFinalOutcomeProof(outcome, validation)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(proof.StatKeys, []uint32{1, 2}) {
		t.Fatalf("stat key order mismatch: %+v", proof.StatKeys)
	}
	if proof.Payload.Stats[0].Stat.Key != 1 || proof.Payload.Stats[1].Stat.Key != 2 {
		t.Fatalf("payload stat order changed: %+v", proof.Payload.Stats)
	}
	ix, err := proof.DevnetValidateInstruction()
	if err != nil {
		t.Fatal(err)
	}
	data, err := ix.Data()
	if err != nil {
		t.Fatal(err)
	}
	if string(data[:8]) != string(ValidateStatV2Discriminator[:]) {
		t.Fatalf("unexpected validation discriminator: %x", data[:8])
	}
}

func TestFinalOutcomeProofRejectsWrongStatOrder(t *testing.T) {
	outcome, err := ExtractFinalOutcome(finalScoreForTest(2, 1), DefaultSoccerFinalOutcomeConfig())
	if err != nil {
		t.Fatal(err)
	}
	validation, err := NewScoresStatValidationV2(
		[]uint32{2, 1},
		finalOutcomeV2ResponseForTestWithKeys(outcome, []uint32{2, 1}),
	)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := NewFinalOutcomeProof(outcome, validation); !errors.Is(err, ErrValidation) {
		t.Fatalf("wrong stat order should be validation error: %v", err)
	}
}

func TestStatTermsForMarketPreservesValidationLeafOrder(t *testing.T) {
	payload := statV2PayloadGolden()
	terms := marketTermsGolden()
	statA, statB, err := StatTermsForMarket(payload, terms)
	if err != nil {
		t.Fatal(err)
	}
	if statA.StatToProve.Key != terms.StatAKey || statB == nil || statB.StatToProve.Key != *terms.StatBKey {
		t.Fatalf("unexpected stat terms: %+v %+v", statA, statB)
	}
	if statA.EventStatRoot != payload.EventStatRoot || statB.EventStatRoot != payload.EventStatRoot {
		t.Fatal("event stat root was not preserved")
	}

	reversed := terms
	reversed.StatAKey = *terms.StatBKey
	reversed.StatBKey = &terms.StatAKey
	statA, statB, err = StatTermsForMarket(payload, reversed)
	if err != nil {
		t.Fatal(err)
	}
	if payload.Stats[0].Stat.Key != terms.StatAKey || payload.Stats[1].Stat.Key != *terms.StatBKey {
		t.Fatalf("payload order changed: %+v", payload.Stats)
	}
	if statA.StatToProve.Key != *terms.StatBKey || statB == nil || statB.StatToProve.Key != terms.StatAKey {
		t.Fatalf("reversed market terms were not selected by key: %+v %+v", statA, statB)
	}
}

func TestLifecyclePlanBuildersMatchLowLevelBuilders(t *testing.T) {
	programID := DevnetProgramPublicKey()
	terms := scoreTermsFromMarketIntent(ScoreMarketFinalOutcome, marketTermsGolden())

	createIntentParams := CreateIntentPlanParams{
		Accounts:      createIntentAccountsGolden(),
		Terms:         terms,
		TermsHash:     hashBytes(100),
		IntentID:      9001,
		DepositAmount: 123_456_789,
		ExpirationTS:  1_781_129_999_999,
		ClaimPeriod:   42,
	}
	createIntentPlan, err := CreateIntentPlan(createIntentParams)
	if err != nil {
		t.Fatal(err)
	}
	createIntentLow, err := createIntentParams.CreateIntentParams()
	if err != nil {
		t.Fatal(err)
	}
	createIntentWant, err := CreateIntentInstruction(programID, createIntentParams.Accounts, createIntentLow)
	if err != nil {
		t.Fatal(err)
	}
	assertSingleInstructionPlan(t, createIntentPlan, "create_intent", createIntentWant)

	createTradeParams := CreateTradePlanParams{
		Accounts:  createTradeAccountsGolden(),
		Terms:     terms,
		TermsHash: hashBytes(110),
		TradeID:   9002,
		StakeA:    111_111,
		StakeB:    222_222,
	}
	createTradePlan, err := CreateTradePlan(createTradeParams)
	if err != nil {
		t.Fatal(err)
	}
	createTradeLow, err := createTradeParams.CreateTradeParams()
	if err != nil {
		t.Fatal(err)
	}
	createTradeWant, err := CreateTradeInstruction(programID, createTradeParams.Accounts, createTradeLow)
	if err != nil {
		t.Fatal(err)
	}
	assertSingleInstructionPlan(t, createTradePlan, "create_trade", createTradeWant)

	executePlan, err := ExecuteMatchPlan(executeMatchAccountsGolden(), executeMatchParamsGolden())
	if err != nil {
		t.Fatal(err)
	}
	executeWant, err := ExecuteMatchInstruction(programID, executeMatchAccountsGolden(), executeMatchParamsGolden())
	if err != nil {
		t.Fatal(err)
	}
	assertSingleInstructionPlan(t, executePlan, "execute_match", executeWant)

	closePlan, err := CloseIntentPlan(closeIntentAccountsGolden())
	if err != nil {
		t.Fatal(err)
	}
	closeWant, err := CloseIntentInstruction(programID, closeIntentAccountsGolden(), CloseIntentParams{})
	if err != nil {
		t.Fatal(err)
	}
	assertSingleInstructionPlan(t, closePlan, "close_intent", closeWant)

	payload := statV2PayloadGolden()
	settleTerms := scoreTermsFromMarketIntent(ScoreMarketTotalGoals, settleTradeMarketTermsForTest())
	settleParams := SettleTradePlanParams{
		Accounts: settleTradeAccountsGolden(),
		TradeID:  9004,
		Terms:    settleTerms,
		Payload:  payload,
	}
	settlePlan, err := SettleTradePlan(settleParams)
	if err != nil {
		t.Fatal(err)
	}
	settleLow, err := settleParams.SettleTradeParams()
	if err != nil {
		t.Fatal(err)
	}
	settleWant, err := SettleTradeInstruction(programID, settleParams.Accounts, settleLow)
	if err != nil {
		t.Fatal(err)
	}
	assertSingleInstructionPlan(t, settlePlan, "settle_trade", settleWant)

	matchedParams := SettleMatchedTradePlanParams{
		Accounts: settleMatchedTradeAccountsGolden(),
		TradeID:  9005,
		Terms:    terms,
		Payload:  payload,
	}
	matchedPlan, err := SettleMatchedTradePlan(matchedParams)
	if err != nil {
		t.Fatal(err)
	}
	matchedLow, err := matchedParams.SettleMatchedTradeParams()
	if err != nil {
		t.Fatal(err)
	}
	matchedWant, err := SettleMatchedTradeInstruction(programID, matchedParams.Accounts, matchedLow)
	if err != nil {
		t.Fatal(err)
	}
	assertSingleInstructionPlan(t, matchedPlan, "settle_matched_trade", matchedWant)

	claimPlan, err := ClaimViaResolutionPlan(claimViaResolutionAccountsGolden(), claimViaResolutionParamsGolden())
	if err != nil {
		t.Fatal(err)
	}
	claimWant, err := ClaimViaResolutionInstruction(programID, claimViaResolutionAccountsGolden(), claimViaResolutionParamsGolden())
	if err != nil {
		t.Fatal(err)
	}
	assertSingleInstructionPlan(t, claimPlan, "claim_via_resolution", claimWant)

	legacyPlan, err := ClaimBatchLegacyPlan(claimBatchLegacyAccountsGolden(), claimBatchLegacyParamsGolden())
	if err != nil {
		t.Fatal(err)
	}
	legacyWant, err := ClaimBatchLegacyInstruction(programID, claimBatchLegacyAccountsGolden(), claimBatchLegacyParamsGolden())
	if err != nil {
		t.Fatal(err)
	}
	assertSingleInstructionPlan(t, legacyPlan, "claim_batch_legacy", legacyWant)

	refundPlan, err := RefundBatchPlan(refundBatchAccountsGolden())
	if err != nil {
		t.Fatal(err)
	}
	refundWant, err := RefundBatchInstruction(programID, refundBatchAccountsGolden(), RefundBatchParams{})
	if err != nil {
		t.Fatal(err)
	}
	assertSingleInstructionPlan(t, refundPlan, "refund_batch", refundWant)

	auditParams := AuditTradeResultPlanParams{
		Accounts: auditTradeResultAccountsGolden(),
		Terms:    terms,
		Payload:  payload,
	}
	auditPlan, err := AuditTradeResultPlan(auditParams)
	if err != nil {
		t.Fatal(err)
	}
	auditLow, err := auditParams.AuditTradeResultParams()
	if err != nil {
		t.Fatal(err)
	}
	auditWant, err := AuditTradeResultInstruction(programID, auditParams.Accounts, auditLow)
	if err != nil {
		t.Fatal(err)
	}
	assertSingleInstructionPlan(t, auditPlan, "audit_trade_result", auditWant)

	strategy, err := terms.Strategy()
	if err != nil {
		t.Fatal(err)
	}
	validatePlan, err := ValidateStatV2Plan(payload, strategy)
	if err != nil {
		t.Fatal(err)
	}
	validateWant, err := DevnetValidateStatV2Instruction(programID, payload, strategy)
	if err != nil {
		t.Fatal(err)
	}
	assertSingleInstructionPlan(t, validatePlan, "validate_stat_v2", validateWant)
}

func finalScoreForTest(p1, p2 int32) Scores {
	return Scores{
		FixtureID: 17_952_170,
		Action:    ScoreActionGameFinalised,
		Seq:       941,
		Ts:        1_781_123_456_789,
		StatusID:  ptrInt32(FinalSettlementStatusID),
		Period:    ptrInt32(FinalSettlementPeriod),
		Stats: map[string]int32{
			"1": p1,
			"2": p2,
		},
	}
}

func finalOutcomeV2ResponseForTest(outcome FinalOutcome) ScoresStatValidationV2Response {
	return finalOutcomeV2ResponseForTestWithKeys(outcome, []uint32{1, 2})
}

func finalOutcomeV2ResponseForTestWithKeys(outcome FinalOutcome, keys []uint32) ScoresStatValidationV2Response {
	values := map[uint32]int32{
		1: outcome.Participant1Goals,
		2: outcome.Participant2Goals,
	}
	stats := make([]ScoreStat, len(keys))
	proofs := make([][]ProofNode, len(keys))
	for i, key := range keys {
		stats[i] = ScoreStat{Key: key, Value: values[key], Period: int32(outcome.Config.Period)}
		proofs[i] = []ProofNode{proof(byte(30+i), i%2 == 0)}
	}
	return ScoresStatValidationV2Response{
		Ts:            outcome.Ts,
		StatsToProve:  stats,
		EventStatRoot: hash32(9),
		Summary: ScoresBatchSummary{
			FixtureID: outcome.FixtureID,
			UpdateStats: UpdateStats{
				UpdateCount:  1,
				MinTimestamp: outcome.Ts,
				MaxTimestamp: outcome.Ts,
			},
			EventStatsSubTreeRoot: hash32(10),
		},
		StatProofs:    proofs,
		SubTreeProof:  []ProofNode{proof(50, false)},
		MainTreeProof: []ProofNode{proof(60, true)},
	}
}

func assertFinalOutcomeStrategy(t *testing.T, side MarketSide, indexA, indexB uint8, comparison Comparison) {
	t.Helper()
	strategy, err := FinalOutcomeStrategyForSide(side)
	if err != nil {
		t.Fatal(err)
	}
	if len(strategy.DiscretePredicates) != 1 {
		t.Fatalf("expected one predicate, got %+v", strategy)
	}
	predicate := strategy.DiscretePredicates[0]
	if predicate.Kind != StatPredicateBinary ||
		predicate.IndexA != indexA ||
		predicate.IndexB != indexB ||
		predicate.Op != Subtract() ||
		predicate.Predicate.Threshold != 0 ||
		predicate.Predicate.Comparison != comparison {
		t.Fatalf("unexpected strategy for %s: %+v", side, predicate)
	}
}

func scoreTermsFromMarketIntent(kind ScoreMarketKind, terms MarketIntentParams) ScoreMarketTerms {
	return ScoreMarketTerms{
		FixtureID: terms.FixtureID,
		Kind:      kind,
		Period:    terms.Period,
		StatAKey:  terms.StatAKey,
		StatBKey:  cloneUint32(terms.StatBKey),
		Predicate: terms.Predicate,
		Op:        cloneBinaryExpression(terms.Op),
		Negation:  terms.Negation,
	}
}

func settleTradeMarketTermsForTest() MarketIntentParams {
	op := Add()
	statBKey := uint32(1002)
	return MarketIntentParams{
		FixtureID: statV2PayloadGolden().FixtureSummary.FixtureID,
		Period:    0,
		StatAKey:  1001,
		StatBKey:  &statBKey,
		Predicate: NewTraderPredicate(1, LessThan()),
		Op:        &op,
	}
}

func assertSingleInstructionPlan(t *testing.T, plan LifecyclePlan, name string, want solana.Instruction) {
	t.Helper()
	if plan.Name != name {
		t.Fatalf("plan name mismatch: got %s want %s", plan.Name, name)
	}
	if len(plan.Instructions) != 1 {
		t.Fatalf("expected one instruction in plan, got %d", len(plan.Instructions))
	}
	if len(plan.CallerResponsibilities) == 0 || len(plan.NextSteps) == 0 {
		t.Fatalf("plan should include caller responsibilities and next steps: %+v", plan)
	}
	assertInstructionEqual(t, plan.Instructions[0], want)
}

func assertInstructionEqual(t *testing.T, got, want solana.Instruction) {
	t.Helper()
	if got.ProgramID() != want.ProgramID() {
		t.Fatalf("program mismatch: got %s want %s", got.ProgramID(), want.ProgramID())
	}
	gotData, err := got.Data()
	if err != nil {
		t.Fatal(err)
	}
	wantData, err := want.Data()
	if err != nil {
		t.Fatal(err)
	}
	if string(gotData) != string(wantData) {
		t.Fatalf("instruction data mismatch\ngot  %x\nwant %x", gotData, wantData)
	}
	gotAccounts := got.Accounts()
	wantAccounts := want.Accounts()
	if len(gotAccounts) != len(wantAccounts) {
		t.Fatalf("account count mismatch: got %d want %d", len(gotAccounts), len(wantAccounts))
	}
	for i := range wantAccounts {
		if *gotAccounts[i] != *wantAccounts[i] {
			t.Fatalf("account %d mismatch: got %+v want %+v", i, *gotAccounts[i], *wantAccounts[i])
		}
	}
}
