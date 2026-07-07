package txline

import (
	"fmt"
	"strconv"

	"github.com/gagliardetto/solana-go"
)

type MarketSide string

const (
	MarketSideParticipant1 MarketSide = "participant1"
	MarketSideParticipant2 MarketSide = "participant2"
	MarketSideDraw         MarketSide = "draw"
)

type ScoreMarketKind string

const (
	ScoreMarketFinalOutcome ScoreMarketKind = "final_outcome"
	ScoreMarketTotalGoals   ScoreMarketKind = "total_goals"
	ScoreMarketSpread       ScoreMarketKind = "spread"
)

type ScoreMarketTerms struct {
	FixtureID int64
	Kind      ScoreMarketKind
	Period    uint16
	StatAKey  uint32
	StatBKey  *uint32
	Predicate TraderPredicate
	Op        *BinaryExpression
	Negation  bool
}

func (t ScoreMarketTerms) MarketIntentParams() (MarketIntentParams, error) {
	if err := t.Validate(); err != nil {
		return MarketIntentParams{}, err
	}
	return MarketIntentParams{
		FixtureID: t.FixtureID,
		Period:    t.Period,
		StatAKey:  t.StatAKey,
		StatBKey:  cloneUint32(t.StatBKey),
		Predicate: t.Predicate,
		Op:        cloneBinaryExpression(t.Op),
		Negation:  t.Negation,
	}, nil
}

func (t ScoreMarketTerms) StatKeys() ([]uint32, error) {
	if err := t.Validate(); err != nil {
		return nil, err
	}
	if t.StatBKey == nil {
		return []uint32{t.StatAKey}, nil
	}
	return []uint32{t.StatAKey, *t.StatBKey}, nil
}

func (t ScoreMarketTerms) Strategy() (NDimensionalStrategy, error) {
	if err := t.Validate(); err != nil {
		return NDimensionalStrategy{}, err
	}
	if t.Negation {
		return NDimensionalStrategy{}, newError(ErrInvalidInput, "V2 strategies do not encode market-term negation")
	}
	statCount := 1
	if t.StatBKey != nil {
		statCount = 2
	}
	builder := NewStrategyBuilder(statCount)
	if t.StatBKey == nil {
		return builder.Single(0, t.Predicate).Build()
	}
	if t.Op == nil {
		return NDimensionalStrategy{}, newError(ErrInvalidInput, "binary score market terms require an operation")
	}
	return builder.Binary(0, 1, *t.Op, t.Predicate).Build()
}

func (t ScoreMarketTerms) Validate() error {
	if t.FixtureID <= 0 {
		return newError(ErrInvalidInput, "score market fixture ID must be greater than zero")
	}
	switch t.Kind {
	case ScoreMarketFinalOutcome:
		return requireBinaryMarket(t, Subtract(), "final outcome")
	case ScoreMarketTotalGoals:
		return requireBinaryMarket(t, Add(), "total goals")
	case ScoreMarketSpread:
		return requireBinaryMarket(t, Subtract(), "spread")
	default:
		return newError(ErrInvalidInput, fmt.Sprintf("unsupported score market kind %q", t.Kind))
	}
}

type FinalOutcomeConfig struct {
	Participant1GoalsStatKey uint32
	Participant2GoalsStatKey uint32
	Period                   uint16
}

func DefaultSoccerFinalOutcomeConfig() FinalOutcomeConfig {
	return FinalOutcomeConfig{
		Participant1GoalsStatKey: 1,
		Participant2GoalsStatKey: 2,
		Period:                   uint16(FinalSettlementPeriod),
	}
}

type FinalOutcome struct {
	FixtureID         int64
	Seq               int32
	Ts                int64
	Participant1Goals int32
	Participant2Goals int32
	Winner            MarketSide
	Config            FinalOutcomeConfig
}

func IsFinalOutcomeRecord(score Scores) bool {
	return score.IsFinalOutcomeRecord()
}

func ExtractFinalOutcome(score Scores, cfg FinalOutcomeConfig) (FinalOutcome, error) {
	cfg = normalizeFinalOutcomeConfig(cfg)
	if !score.IsFinalOutcomeRecord() {
		return FinalOutcome{}, newError(ErrValidation, "score record is not a final outcome record; expected action=game_finalised, statusId=100, period=100")
	}
	if err := EnsurePositiveSeq(score.Seq); err != nil {
		return FinalOutcome{}, err
	}
	p1, err := scoreStatValue(score, cfg.Participant1GoalsStatKey)
	if err != nil {
		return FinalOutcome{}, err
	}
	p2, err := scoreStatValue(score, cfg.Participant2GoalsStatKey)
	if err != nil {
		return FinalOutcome{}, err
	}
	winner := MarketSideDraw
	if p1 > p2 {
		winner = MarketSideParticipant1
	} else if p2 > p1 {
		winner = MarketSideParticipant2
	}
	return FinalOutcome{
		FixtureID:         score.FixtureID,
		Seq:               score.Seq,
		Ts:                score.Ts,
		Participant1Goals: p1,
		Participant2Goals: p2,
		Winner:            winner,
		Config:            cfg,
	}, nil
}

func FindFinalOutcome(scores []Scores, cfg FinalOutcomeConfig) (FinalOutcome, error) {
	for _, score := range scores {
		if score.IsFinalOutcomeRecord() {
			return ExtractFinalOutcome(score, cfg)
		}
	}
	return FinalOutcome{}, newError(ErrValidation, "no final outcome score record found")
}

func FinalOutcomeStatKeys(cfg FinalOutcomeConfig) []uint32 {
	cfg = normalizeFinalOutcomeConfig(cfg)
	return []uint32{cfg.Participant1GoalsStatKey, cfg.Participant2GoalsStatKey}
}

func FinalOutcomeMarketTerms(fixtureID int64, side MarketSide, cfg FinalOutcomeConfig) (ScoreMarketTerms, error) {
	cfg = normalizeFinalOutcomeConfig(cfg)
	predicate := NewTraderPredicate(0, GreaterThan())
	op := Subtract()
	terms := ScoreMarketTerms{
		FixtureID: fixtureID,
		Kind:      ScoreMarketFinalOutcome,
		Period:    cfg.Period,
		StatAKey:  cfg.Participant1GoalsStatKey,
		StatBKey:  &cfg.Participant2GoalsStatKey,
		Predicate: predicate,
		Op:        &op,
	}
	switch side {
	case MarketSideParticipant1:
	case MarketSideParticipant2:
		terms.StatAKey = cfg.Participant2GoalsStatKey
		terms.StatBKey = &cfg.Participant1GoalsStatKey
	case MarketSideDraw:
		terms.Predicate = NewTraderPredicate(0, EqualTo())
	default:
		return ScoreMarketTerms{}, newError(ErrInvalidInput, fmt.Sprintf("unsupported final outcome side %q", side))
	}
	if err := terms.Validate(); err != nil {
		return ScoreMarketTerms{}, err
	}
	return terms, nil
}

func TotalGoalsMarketTerms(fixtureID int64, threshold int32, comparison Comparison, cfg FinalOutcomeConfig) (ScoreMarketTerms, error) {
	cfg = normalizeFinalOutcomeConfig(cfg)
	op := Add()
	terms := ScoreMarketTerms{
		FixtureID: fixtureID,
		Kind:      ScoreMarketTotalGoals,
		Period:    cfg.Period,
		StatAKey:  cfg.Participant1GoalsStatKey,
		StatBKey:  &cfg.Participant2GoalsStatKey,
		Predicate: NewTraderPredicate(threshold, comparison),
		Op:        &op,
	}
	if err := terms.Validate(); err != nil {
		return ScoreMarketTerms{}, err
	}
	return terms, nil
}

func SpreadMarketTerms(fixtureID int64, side MarketSide, threshold int32, comparison Comparison, cfg FinalOutcomeConfig) (ScoreMarketTerms, error) {
	cfg = normalizeFinalOutcomeConfig(cfg)
	op := Subtract()
	terms := ScoreMarketTerms{
		FixtureID: fixtureID,
		Kind:      ScoreMarketSpread,
		Period:    cfg.Period,
		StatAKey:  cfg.Participant1GoalsStatKey,
		StatBKey:  &cfg.Participant2GoalsStatKey,
		Predicate: NewTraderPredicate(threshold, comparison),
		Op:        &op,
	}
	switch side {
	case MarketSideParticipant1:
	case MarketSideParticipant2:
		terms.StatAKey = cfg.Participant2GoalsStatKey
		terms.StatBKey = &cfg.Participant1GoalsStatKey
	case MarketSideDraw:
		return ScoreMarketTerms{}, newError(ErrInvalidInput, "spread markets must choose participant1 or participant2")
	default:
		return ScoreMarketTerms{}, newError(ErrInvalidInput, fmt.Sprintf("unsupported spread side %q", side))
	}
	if err := terms.Validate(); err != nil {
		return ScoreMarketTerms{}, err
	}
	return terms, nil
}

func FinalOutcomeStrategy(outcome FinalOutcome) (NDimensionalStrategy, error) {
	return FinalOutcomeStrategyForSide(outcome.Winner)
}

func FinalOutcomeStrategyForSide(side MarketSide) (NDimensionalStrategy, error) {
	switch side {
	case MarketSideParticipant1:
		return NewStrategyBuilder(2).
			Binary(0, 1, Subtract(), NewTraderPredicate(0, GreaterThan())).
			Build()
	case MarketSideParticipant2:
		return NewStrategyBuilder(2).
			Binary(1, 0, Subtract(), NewTraderPredicate(0, GreaterThan())).
			Build()
	case MarketSideDraw:
		return NewStrategyBuilder(2).
			Binary(0, 1, Subtract(), NewTraderPredicate(0, EqualTo())).
			Build()
	default:
		return NDimensionalStrategy{}, newError(ErrInvalidInput, fmt.Sprintf("unsupported final outcome side %q", side))
	}
}

type FinalOutcomeProof struct {
	Outcome  FinalOutcome
	StatKeys []uint32
	Payload  StatValidationInput
	Strategy NDimensionalStrategy
}

func NewFinalOutcomeProof(outcome FinalOutcome, validation *ScoresStatValidationV2) (FinalOutcomeProof, error) {
	if validation == nil {
		return FinalOutcomeProof{}, newError(ErrInvalidInput, "final outcome proof requires a V2 stat validation payload")
	}
	statKeys := FinalOutcomeStatKeys(outcome.Config)
	if err := requireStatKeyOrder(validation.RequestedStatKeys(), statKeys); err != nil {
		return FinalOutcomeProof{}, err
	}
	payload := validation.ToValidationInput()
	if err := validateFinalOutcomePayload(outcome, payload); err != nil {
		return FinalOutcomeProof{}, err
	}
	strategy, err := FinalOutcomeStrategy(outcome)
	if err != nil {
		return FinalOutcomeProof{}, err
	}
	return FinalOutcomeProof{
		Outcome:  outcome,
		StatKeys: statKeys,
		Payload:  payload,
		Strategy: strategy,
	}, nil
}

func (p FinalOutcomeProof) DevnetValidateInstruction() (solana.Instruction, error) {
	return DevnetValidateStatV2Instruction(DevnetProgramPublicKey(), p.Payload, p.Strategy)
}

func StatTermsForMarket(payload StatValidationInput, terms MarketIntentParams) (StatTermInput, *StatTermInput, error) {
	if payload.FixtureSummary.FixtureID != terms.FixtureID {
		return StatTermInput{}, nil, newError(ErrValidation, "validation payload fixture ID does not match market terms")
	}
	statA, err := statTermForKey(payload, terms.StatAKey)
	if err != nil {
		return StatTermInput{}, nil, err
	}
	if terms.StatBKey == nil {
		return statA, nil, nil
	}
	statB, err := statTermForKey(payload, *terms.StatBKey)
	if err != nil {
		return StatTermInput{}, nil, err
	}
	return statA, &statB, nil
}

type LifecyclePlan struct {
	Name                   string
	Instructions           []solana.Instruction
	CallerResponsibilities []string
	NextSteps              []string
}

type CreateIntentPlanParams struct {
	Accounts      CreateIntentAccounts
	Terms         ScoreMarketTerms
	TermsHash     [32]byte
	IntentID      uint64
	DepositAmount uint64
	ExpirationTS  int64
	ClaimPeriod   uint16
}

func (p CreateIntentPlanParams) CreateIntentParams() (CreateIntentParams, error) {
	if _, err := p.Terms.MarketIntentParams(); err != nil {
		return CreateIntentParams{}, err
	}
	return CreateIntentParams{
		IntentID:      p.IntentID,
		TermsHash:     p.TermsHash,
		DepositAmount: p.DepositAmount,
		ExpirationTS:  p.ExpirationTS,
		ClaimPeriod:   p.ClaimPeriod,
		FixtureID:     p.Terms.FixtureID,
	}, nil
}

func CreateIntentPlan(params CreateIntentPlanParams) (LifecyclePlan, error) {
	ixParams, err := params.CreateIntentParams()
	if err != nil {
		return LifecyclePlan{}, err
	}
	ix, err := CreateIntentInstruction(DevnetProgramPublicKey(), params.Accounts, ixParams)
	if err != nil {
		return LifecyclePlan{}, err
	}
	return singleInstructionPlan(
		"create_intent", ix,
		[]string{"Provide the order intent, vault, token mint, token program, treasury, and maker token accounts from the coordinating application."},
		[]string{"Maker signs and submits the instruction, then waits for a taker intent or an explicit close."},
	), nil
}

type CreateTradePlanParams struct {
	Accounts  CreateTradeAccounts
	Terms     ScoreMarketTerms
	TermsHash [32]byte
	TradeID   uint64
	StakeA    uint64
	StakeB    uint64
}

func (p CreateTradePlanParams) CreateTradeParams() (CreateTradeParams, error) {
	if _, err := p.Terms.MarketIntentParams(); err != nil {
		return CreateTradeParams{}, err
	}
	return CreateTradeParams{
		TradeID:        p.TradeID,
		StakeA:         p.StakeA,
		StakeB:         p.StakeB,
		TradeTermsHash: p.TermsHash,
	}, nil
}

func CreateTradePlan(params CreateTradePlanParams) (LifecyclePlan, error) {
	ixParams, err := params.CreateTradeParams()
	if err != nil {
		return LifecyclePlan{}, err
	}
	ix, err := CreateTradeInstruction(DevnetProgramPublicKey(), params.Accounts, ixParams)
	if err != nil {
		return LifecyclePlan{}, err
	}
	return singleInstructionPlan(
		"create_trade", ix,
		[]string{"Provide both traders, token accounts, escrow accounts, stake mint, token program, and treasury accounts explicitly."},
		[]string{"Both traders sign the direct trade instruction before score observation and settlement."},
	), nil
}

func ExecuteMatchPlan(accounts ExecuteMatchAccounts, params ExecuteMatchParams) (LifecyclePlan, error) {
	ix, err := ExecuteMatchInstruction(DevnetProgramPublicKey(), accounts, params)
	if err != nil {
		return LifecyclePlan{}, err
	}
	return singleInstructionPlan(
		"execute_match", ix,
		[]string{"Provide maker/taker intents, their vaults, matched-trade account, trade vault, token mint, token program, and solver signer."},
		[]string{"After matching, observe scores and settle the matched trade with a real validation payload."},
	), nil
}

func CloseIntentPlan(accounts CloseIntentAccounts) (LifecyclePlan, error) {
	ix, err := CloseIntentInstruction(DevnetProgramPublicKey(), accounts, CloseIntentParams{})
	if err != nil {
		return LifecyclePlan{}, err
	}
	return singleInstructionPlan(
		"close_intent", ix,
		[]string{"Provide the intent, vault, maker token account, token mint, token program, treasury, and close authority."},
		[]string{"Use this path when the intent expires or the application chooses to cancel it before matching."},
	), nil
}

type SettleTradePlanParams struct {
	Accounts SettleTradeAccounts
	TradeID  uint64
	Terms    ScoreMarketTerms
	Payload  StatValidationInput
}

func (p SettleTradePlanParams) SettleTradeParams() (SettleTradeParams, error) {
	terms, err := p.Terms.MarketIntentParams()
	if err != nil {
		return SettleTradeParams{}, err
	}
	if terms.Negation {
		return SettleTradeParams{}, newError(ErrInvalidInput, "settle_trade does not encode market-term negation")
	}
	statA, statB, err := StatTermsForMarket(p.Payload, terms)
	if err != nil {
		return SettleTradeParams{}, err
	}
	return SettleTradeParams{
		TradeID:        p.TradeID,
		Ts:             p.Payload.Ts,
		FixtureSummary: p.Payload.FixtureSummary,
		FixtureProof:   append([]ProofNode(nil), p.Payload.FixtureProof...),
		MainTreeProof:  append([]ProofNode(nil), p.Payload.MainTreeProof...),
		Predicate:      terms.Predicate,
		StatA:          statA,
		StatB:          statB,
		Op:             cloneBinaryExpression(terms.Op),
	}, nil
}

func SettleTradePlan(params SettleTradePlanParams) (LifecyclePlan, error) {
	ixParams, err := params.SettleTradeParams()
	if err != nil {
		return LifecyclePlan{}, err
	}
	ix, err := SettleTradeInstruction(DevnetProgramPublicKey(), params.Accounts, ixParams)
	if err != nil {
		return LifecyclePlan{}, err
	}
	return singleInstructionPlan(
		"settle_trade", ix,
		[]string{"Provide the direct trade escrow, escrow vault, winner token account, token mint, token program, treasury, and daily scores root account."},
		[]string{"Winner signs after the score proof payload has been fetched for the observed final score sequence."},
	), nil
}

type SettleMatchedTradePlanParams struct {
	Accounts SettleMatchedTradeAccounts
	TradeID  uint64
	Terms    ScoreMarketTerms
	Payload  StatValidationInput
}

func (p SettleMatchedTradePlanParams) SettleMatchedTradeParams() (SettleMatchedTradeParams, error) {
	terms, err := p.Terms.MarketIntentParams()
	if err != nil {
		return SettleMatchedTradeParams{}, err
	}
	statA, statB, err := StatTermsForMarket(p.Payload, terms)
	if err != nil {
		return SettleMatchedTradeParams{}, err
	}
	return SettleMatchedTradeParams{
		TradeID:        p.TradeID,
		Ts:             p.Payload.Ts,
		FixtureSummary: p.Payload.FixtureSummary,
		FixtureProof:   append([]ProofNode(nil), p.Payload.FixtureProof...),
		MainTreeProof:  append([]ProofNode(nil), p.Payload.MainTreeProof...),
		StatA:          statA,
		StatB:          statB,
		Terms:          terms,
	}, nil
}

func SettleMatchedTradePlan(params SettleMatchedTradePlanParams) (LifecyclePlan, error) {
	ixParams, err := params.SettleMatchedTradeParams()
	if err != nil {
		return LifecyclePlan{}, err
	}
	ix, err := SettleMatchedTradeInstruction(DevnetProgramPublicKey(), params.Accounts, ixParams)
	if err != nil {
		return LifecyclePlan{}, err
	}
	return singleInstructionPlan(
		"settle_matched_trade", ix,
		[]string{"Provide the matched trade, trade vault, winner token account, token mint, token program, treasury, and daily scores root account."},
		[]string{"Winner signs after matching and proof retrieval; the SDK does not infer the matched trade account."},
	), nil
}

func ClaimViaResolutionPlan(accounts ClaimViaResolutionAccounts, params ClaimViaResolutionParams) (LifecyclePlan, error) {
	ix, err := ClaimViaResolutionInstruction(DevnetProgramPublicKey(), accounts, params)
	if err != nil {
		return LifecyclePlan{}, err
	}
	return singleInstructionPlan(
		"claim_via_resolution", ix,
		[]string{"Provide the daily resolution root, matched trade, trade vault, winner token account, token program, and winner signer."},
		[]string{"Use only with a real published resolution proof from the coordinating application or backend."},
	), nil
}

func ClaimBatchLegacyPlan(accounts ClaimBatchLegacyAccounts, params ClaimBatchLegacyParams) (LifecyclePlan, error) {
	ix, err := ClaimBatchLegacyInstruction(DevnetProgramPublicKey(), accounts, params)
	if err != nil {
		return LifecyclePlan{}, err
	}
	return singleInstructionPlan(
		"claim_batch_legacy", ix,
		[]string{"Provide the daily resolution root, stake mint, token program, system program, and payer signer."},
		[]string{"Use only when the application has a compatible legacy batch resolution proof."},
	), nil
}

func RefundBatchPlan(accounts RefundBatchAccounts) (LifecyclePlan, error) {
	ix, err := RefundBatchInstruction(DevnetProgramPublicKey(), accounts, RefundBatchParams{})
	if err != nil {
		return LifecyclePlan{}, err
	}
	return singleInstructionPlan(
		"refund_batch", ix,
		[]string{"Provide the payer, stake mint, token program, and system program explicitly."},
		[]string{"Use for the public refund path exposed by the Devnet IDL when the application has determined refund eligibility."},
	), nil
}

type AuditTradeResultPlanParams struct {
	Accounts AuditTradeResultAccounts
	Terms    ScoreMarketTerms
	Payload  StatValidationInput
}

func (p AuditTradeResultPlanParams) AuditTradeResultParams() (AuditTradeResultParams, error) {
	terms, err := p.Terms.MarketIntentParams()
	if err != nil {
		return AuditTradeResultParams{}, err
	}
	statA, statB, err := StatTermsForMarket(p.Payload, terms)
	if err != nil {
		return AuditTradeResultParams{}, err
	}
	return AuditTradeResultParams{
		Terms:          terms,
		FixtureSummary: p.Payload.FixtureSummary,
		MainTreeProof:  append([]ProofNode(nil), p.Payload.MainTreeProof...),
		FixtureProof:   append([]ProofNode(nil), p.Payload.FixtureProof...),
		StatA:          statA,
		StatB:          statB,
		Ts:             p.Payload.Ts,
	}, nil
}

func AuditTradeResultPlan(params AuditTradeResultPlanParams) (LifecyclePlan, error) {
	ixParams, err := params.AuditTradeResultParams()
	if err != nil {
		return LifecyclePlan{}, err
	}
	ix, err := AuditTradeResultInstruction(DevnetProgramPublicKey(), params.Accounts, ixParams)
	if err != nil {
		return LifecyclePlan{}, err
	}
	return singleInstructionPlan(
		"audit_trade_result", ix,
		[]string{"Provide the payer signer and daily scores root account for the validation payload timestamp."},
		[]string{"Use this path to check market terms against a real score proof before or alongside settlement."},
	), nil
}

func ValidateStatV2Plan(payload StatValidationInput, strategy NDimensionalStrategy) (LifecyclePlan, error) {
	ix, err := DevnetValidateStatV2Instruction(DevnetProgramPublicKey(), payload, strategy)
	if err != nil {
		return LifecyclePlan{}, err
	}
	return singleInstructionPlan(
		"validate_stat_v2", ix,
		[]string{"Fetch the proof from /api/scores/stat-validation with the same ordered stat keys used by the strategy."},
		[]string{"Simulate or include the instruction in a transaction only after reviewing the payload and strategy indexes."},
	), nil
}

func singleInstructionPlan(name string, ix solana.Instruction, responsibilities, nextSteps []string) LifecyclePlan {
	return LifecyclePlan{
		Name:                   name,
		Instructions:           []solana.Instruction{ix},
		CallerResponsibilities: append([]string(nil), responsibilities...),
		NextSteps:              append([]string(nil), nextSteps...),
	}
}

func normalizeFinalOutcomeConfig(cfg FinalOutcomeConfig) FinalOutcomeConfig {
	defaults := DefaultSoccerFinalOutcomeConfig()
	if cfg.Participant1GoalsStatKey == 0 {
		cfg.Participant1GoalsStatKey = defaults.Participant1GoalsStatKey
	}
	if cfg.Participant2GoalsStatKey == 0 {
		cfg.Participant2GoalsStatKey = defaults.Participant2GoalsStatKey
	}
	if cfg.Period == 0 {
		cfg.Period = defaults.Period
	}
	return cfg
}

func scoreStatValue(score Scores, key uint32) (int32, error) {
	if score.Stats == nil {
		return 0, newError(ErrValidation, "score record has no stats map")
	}
	value, ok := score.Stats[strconv.FormatUint(uint64(key), 10)]
	if !ok {
		return 0, newError(ErrValidation, fmt.Sprintf("score record is missing stat key %d", key))
	}
	return value, nil
}

func requireBinaryMarket(t ScoreMarketTerms, expectedOp BinaryExpression, label string) error {
	if t.StatBKey == nil {
		return newError(ErrInvalidInput, label+" market requires a second stat key")
	}
	if t.Op == nil {
		return newError(ErrInvalidInput, label+" market requires a binary operation")
	}
	if *t.Op != expectedOp {
		return newError(ErrInvalidInput, label+" market uses an unsupported binary operation")
	}
	return nil
}

func requireStatKeyOrder(got, want []uint32) error {
	if len(got) != len(want) {
		return newError(ErrValidation, fmt.Sprintf("stat key count %d does not match expected %d", len(got), len(want)))
	}
	for i := range want {
		if got[i] != want[i] {
			return newError(ErrValidation, fmt.Sprintf("stat key %d is %d, expected %d", i, got[i], want[i]))
		}
	}
	return nil
}

func validateFinalOutcomePayload(outcome FinalOutcome, payload StatValidationInput) error {
	cfg := normalizeFinalOutcomeConfig(outcome.Config)
	if payload.FixtureSummary.FixtureID != outcome.FixtureID {
		return newError(ErrValidation, "final outcome proof fixture ID does not match outcome")
	}
	if len(payload.Stats) != 2 {
		return newError(ErrValidation, fmt.Sprintf("final outcome proof must contain exactly 2 stats, got %d", len(payload.Stats)))
	}
	expected := []struct {
		key    uint32
		value  int32
		period int32
	}{
		{key: cfg.Participant1GoalsStatKey, value: outcome.Participant1Goals, period: int32(cfg.Period)},
		{key: cfg.Participant2GoalsStatKey, value: outcome.Participant2Goals, period: int32(cfg.Period)},
	}
	for i, stat := range payload.Stats {
		if stat.Stat.Key != expected[i].key {
			return newError(ErrValidation, fmt.Sprintf("final outcome proof stat %d key %d does not match expected key %d", i, stat.Stat.Key, expected[i].key))
		}
		if stat.Stat.Value != expected[i].value {
			return newError(ErrValidation, fmt.Sprintf("final outcome proof stat %d value %d does not match observed value %d", i, stat.Stat.Value, expected[i].value))
		}
		if stat.Stat.Period != expected[i].period {
			return newError(ErrValidation, fmt.Sprintf("final outcome proof stat %d period %d does not match expected period %d", i, stat.Stat.Period, expected[i].period))
		}
	}
	return nil
}

func statTermFromLeaf(root [32]byte, leaf StatLeafInput) StatTermInput {
	return StatTermInput{
		StatToProve:   leaf.Stat,
		EventStatRoot: root,
		StatProof:     append([]ProofNode(nil), leaf.StatProof...),
	}
}

func statTermForKey(payload StatValidationInput, key uint32) (StatTermInput, error) {
	var found *StatLeafInput
	for idx := range payload.Stats {
		if payload.Stats[idx].Stat.Key != key {
			continue
		}
		if found != nil {
			return StatTermInput{}, newError(ErrValidation, fmt.Sprintf("validation payload contains duplicate stat key %d", key))
		}
		found = &payload.Stats[idx]
	}
	if found == nil {
		return StatTermInput{}, newError(ErrValidation, fmt.Sprintf("validation payload is missing stat key %d", key))
	}
	return statTermFromLeaf(payload.EventStatRoot, *found), nil
}

func cloneUint32(value *uint32) *uint32 {
	if value == nil {
		return nil
	}
	copy := *value
	return &copy
}

func cloneBinaryExpression(value *BinaryExpression) *BinaryExpression {
	if value == nil {
		return nil
	}
	copy := *value
	return &copy
}
