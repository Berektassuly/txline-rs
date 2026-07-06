package txline

import "encoding/json"

type ExtraFields map[string]json.RawMessage

const (
	FixtureGameStateScheduled = 1
	FixtureGameStateCancelled = 6
	ScoreActionGameFinalised  = "game_finalised"
	FinalSettlementStatusID   = 100
	FinalSettlementPeriod     = 100
)

type Fixture struct {
	Ts                 int64           `json:"Ts"`
	StartTime          int64           `json:"StartTime"`
	Competition        string          `json:"Competition"`
	CompetitionID      int32           `json:"CompetitionId"`
	FixtureGroupID     int32           `json:"FixtureGroupId"`
	Participant1ID     int32           `json:"Participant1Id"`
	Participant1       string          `json:"Participant1"`
	Participant2ID     int32           `json:"Participant2Id"`
	Participant2       string          `json:"Participant2"`
	FixtureID          int64           `json:"FixtureId"`
	Participant1IsHome bool            `json:"Participant1IsHome"`
	GameState          *int32          `json:"GameState,omitempty"`
	Extra              json.RawMessage `json:"-"`
}

func (f Fixture) IsScheduled() bool {
	return f.GameState != nil && *f.GameState == FixtureGameStateScheduled
}

func (f Fixture) IsCancelled() bool {
	return f.GameState != nil && *f.GameState == FixtureGameStateCancelled
}

type OddsPayload struct {
	FixtureID        int64           `json:"FixtureId"`
	MessageID        string          `json:"MessageId"`
	Ts               int64           `json:"Ts"`
	Bookmaker        string          `json:"Bookmaker"`
	BookmakerID      int32           `json:"BookmakerId"`
	SuperOddsType    string          `json:"SuperOddsType"`
	GameState        *string         `json:"GameState,omitempty"`
	InRunning        bool            `json:"InRunning"`
	MarketParameters *string         `json:"MarketParameters,omitempty"`
	MarketPeriod     *string         `json:"MarketPeriod,omitempty"`
	PriceNames       []string        `json:"PriceNames,omitempty"`
	Prices           []int32         `json:"Prices,omitempty"`
	Pct              []string        `json:"Pct,omitempty"`
	Extra            json.RawMessage `json:"-"`
}

type PlayerStats struct {
	Goals           *int32 `json:"goals,omitempty"`
	OwnGoals        *int32 `json:"ownGoals,omitempty"`
	PenaltyAttempts *int32 `json:"penaltyAttempts,omitempty"`
	PenaltyGoals    *int32 `json:"penaltyGoals,omitempty"`
	RedCards        *int32 `json:"redCards,omitempty"`
	Shots           *int32 `json:"shots,omitempty"`
	YellowCards     *int32 `json:"yellowCards,omitempty"`
}

type PlayerStatsForParticipants struct {
	Participant1 map[int64]PlayerStats `json:"Participant1,omitempty"`
	Participant2 map[int64]PlayerStats `json:"Participant2,omitempty"`
}

type Scores struct {
	FixtureID             int64                       `json:"fixtureId"`
	GameState             string                      `json:"gameState"`
	StartTime             int64                       `json:"startTime"`
	IsTeam                bool                        `json:"isTeam"`
	FixtureGroupID        int32                       `json:"fixtureGroupId"`
	CompetitionID         int32                       `json:"competitionId"`
	CountryID             int32                       `json:"countryId"`
	SportID               int32                       `json:"sportId"`
	Participant1IsHome    bool                        `json:"participant1IsHome"`
	Participant2ID        int32                       `json:"participant2Id"`
	Participant1ID        int32                       `json:"participant1Id"`
	Action                string                      `json:"action"`
	ID                    int32                       `json:"id"`
	Ts                    int64                       `json:"ts"`
	ConnectionID          int64                       `json:"connectionId"`
	Seq                   int32                       `json:"seq"`
	StatusID              *int32                      `json:"statusId,omitempty"`
	Period                *int32                      `json:"period,omitempty"`
	CoverageSecondaryData *bool                       `json:"coverageSecondaryData,omitempty"`
	CoverageType          *string                     `json:"coverageType,omitempty"`
	Confirmed             *bool                       `json:"confirmed,omitempty"`
	Participant           *int32                      `json:"participant,omitempty"`
	Possession            *int32                      `json:"possession,omitempty"`
	Stats                 map[string]int32            `json:"stats,omitempty"`
	PlayerStats           *PlayerStatsForParticipants `json:"PlayerStats,omitempty"`
	Extra                 json.RawMessage             `json:"-"`
}

func (s Scores) IsFinalOutcomeRecord() bool {
	return s.Action == ScoreActionGameFinalised &&
		s.StatusID != nil && *s.StatusID == FinalSettlementStatusID &&
		s.Period != nil && *s.Period == FinalSettlementPeriod
}

type UpdateStats struct {
	UpdateCount  int32 `json:"updateCount"`
	MinTimestamp int64 `json:"minTimestamp"`
	MaxTimestamp int64 `json:"maxTimestamp"`
}

type BatchMetadata struct {
	TotalUpdateCount    int32 `json:"totalUpdateCount"`
	NumUniqueFixtures   int32 `json:"numUniqueFixtures"`
	OverallBatchStartTs int64 `json:"overallBatchStartTs"`
	OverallBatchEndTs   int64 `json:"overallBatchEndTs"`
}

type FixtureBatchSummary struct {
	FixtureID         int64       `json:"fixtureId"`
	CompetitionID     int32       `json:"competitionId"`
	Competition       string      `json:"competition"`
	UpdateStats       UpdateStats `json:"updateStats"`
	UpdateSubTreeRoot Hash32      `json:"updateSubTreeRoot"`
}

type FixtureValidation struct {
	Snapshot      Fixture             `json:"snapshot"`
	Summary       FixtureBatchSummary `json:"summary"`
	SubTreeProof  []ProofNode         `json:"subTreeProof,omitempty"`
	MainTreeProof []ProofNode         `json:"mainTreeProof,omitempty"`
}

type FixtureBatchValidation struct {
	Metadata BatchMetadata `json:"metadata"`
	Proof    []ProofNode   `json:"proof,omitempty"`
}

type OddsBatchSummary struct {
	FixtureID       int64       `json:"fixtureId"`
	UpdateStats     UpdateStats `json:"updateStats"`
	OddsSubTreeRoot Hash32      `json:"oddsSubTreeRoot"`
}

type OddsValidation struct {
	Odds          OddsPayload      `json:"odds"`
	Summary       OddsBatchSummary `json:"summary"`
	SubTreeProof  []ProofNode      `json:"subTreeProof,omitempty"`
	MainTreeProof []ProofNode      `json:"mainTreeProof,omitempty"`
}

type PurchaseQuoteRequest struct {
	BuyerPubkey  string `json:"buyerPubkey"`
	TxlineAmount uint64 `json:"txlineAmount"`
}

type PurchaseQuoteResponse struct {
	TransactionBase64 string  `json:"transactionBase64"`
	BaseUSDTCost      float64 `json:"baseUsdtCost"`
	FeeUSDTAmount     float64 `json:"feeUsdtAmount"`
	TotalUSDTCharged  float64 `json:"totalUsdtCharged"`
}
