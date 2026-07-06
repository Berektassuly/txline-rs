package txline

import "fmt"

type ScoreStat struct {
	Key    uint32 `json:"key"`
	Value  int32  `json:"value"`
	Period int32  `json:"period"`
}

type ScoresBatchSummary struct {
	FixtureID             int64       `json:"fixtureId"`
	UpdateStats           UpdateStats `json:"updateStats"`
	EventStatsSubTreeRoot Hash32      `json:"eventStatsSubTreeRoot"`
}

type ScoresStatValidation struct {
	Ts            int64              `json:"ts"`
	StatToProve   ScoreStat          `json:"statToProve"`
	EventStatRoot Hash32             `json:"eventStatRoot"`
	Summary       ScoresBatchSummary `json:"summary"`
	StatProof     []ProofNode        `json:"statProof"`
	SubTreeProof  []ProofNode        `json:"subTreeProof"`
	MainTreeProof []ProofNode        `json:"mainTreeProof"`
	StatToProve2  *ScoreStat         `json:"statToProve2,omitempty"`
	StatProof2    []ProofNode        `json:"statProof2,omitempty"`
}

type FixtureSummaryInput struct {
	FixtureID         int64
	UpdateCount       int32
	MinTimestamp      int64
	MaxTimestamp      int64
	EventsSubTreeRoot [32]byte
}

type StatTermInput struct {
	StatToProve   ScoreStat
	EventStatRoot [32]byte
	StatProof     []ProofNode
}

func (s ScoresStatValidation) FixtureSummaryInput() FixtureSummaryInput {
	return FixtureSummaryInput{
		FixtureID:         s.Summary.FixtureID,
		UpdateCount:       s.Summary.UpdateStats.UpdateCount,
		MinTimestamp:      s.Summary.UpdateStats.MinTimestamp,
		MaxTimestamp:      s.Summary.UpdateStats.MaxTimestamp,
		EventsSubTreeRoot: s.Summary.EventStatsSubTreeRoot.Bytes(),
	}
}

func (s ScoresStatValidation) PrimaryStatTerm() StatTermInput {
	return StatTermInput{
		StatToProve:   s.StatToProve,
		EventStatRoot: s.EventStatRoot.Bytes(),
		StatProof:     append([]ProofNode(nil), s.StatProof...),
	}
}

func (s ScoresStatValidation) SecondaryStatTerm() (*StatTermInput, error) {
	hasStat := s.StatToProve2 != nil
	hasProof := s.StatProof2 != nil
	switch {
	case hasStat && hasProof:
		return &StatTermInput{
			StatToProve:   *s.StatToProve2,
			EventStatRoot: s.EventStatRoot.Bytes(),
			StatProof:     append([]ProofNode(nil), s.StatProof2...),
		}, nil
	case !hasStat && !hasProof:
		return nil, nil
	default:
		return nil, &Error{Kind: ErrValidation, Msg: "legacy response contains only one of statToProve2/statProof2"}
	}
}

func (s ScoresStatValidation) EpochDay() (uint16, error) {
	return TimestampMSToEpochDay(s.Summary.UpdateStats.MinTimestamp)
}

type ScoresStatValidationV2Response struct {
	Ts            int64              `json:"ts"`
	StatsToProve  []ScoreStat        `json:"statsToProve"`
	EventStatRoot Hash32             `json:"eventStatRoot"`
	Summary       ScoresBatchSummary `json:"summary"`
	StatProofs    [][]ProofNode      `json:"statProofs"`
	SubTreeProof  []ProofNode        `json:"subTreeProof"`
	MainTreeProof []ProofNode        `json:"mainTreeProof"`
}

type ScoresStatValidationV2 struct {
	requestedStatKeys []uint32
	response          ScoresStatValidationV2Response
}

type StatLeafInput struct {
	Stat      ScoreStat
	StatProof []ProofNode
}

type StatValidationInput struct {
	Ts             int64
	FixtureSummary FixtureSummaryInput
	FixtureProof   []ProofNode
	MainTreeProof  []ProofNode
	EventStatRoot  [32]byte
	Stats          []StatLeafInput
}

func NewScoresStatValidationV2(requestedStatKeys []uint32, response ScoresStatValidationV2Response) (*ScoresStatValidationV2, error) {
	if len(requestedStatKeys) == 0 {
		return nil, &Error{Kind: ErrInvalidInput, Msg: "V2 stat validation requires at least one stat key"}
	}
	if len(response.StatsToProve) != len(requestedStatKeys) {
		return nil, &Error{
			Kind: ErrValidation,
			Msg:  fmt.Sprintf("statsToProve length %d does not match requested statKeys length %d", len(response.StatsToProve), len(requestedStatKeys)),
		}
	}
	for idx, stat := range response.StatsToProve {
		if stat.Key != requestedStatKeys[idx] {
			return nil, &Error{
				Kind: ErrValidation,
				Msg:  fmt.Sprintf("statsToProve[%d].key %d does not match requested statKeys[%d] %d", idx, stat.Key, idx, requestedStatKeys[idx]),
			}
		}
	}
	if len(response.StatProofs) != len(response.StatsToProve) {
		return nil, &Error{
			Kind: ErrValidation,
			Msg:  fmt.Sprintf("statProofs length %d does not match statsToProve length %d", len(response.StatProofs), len(response.StatsToProve)),
		}
	}
	return &ScoresStatValidationV2{
		requestedStatKeys: append([]uint32(nil), requestedStatKeys...),
		response:          response,
	}, nil
}

func (s *ScoresStatValidationV2) RequestedStatKeys() []uint32 {
	return append([]uint32(nil), s.requestedStatKeys...)
}

func (s *ScoresStatValidationV2) StatsToProve() []ScoreStat {
	return append([]ScoreStat(nil), s.response.StatsToProve...)
}

func (s *ScoresStatValidationV2) StatProofs() [][]ProofNode {
	out := make([][]ProofNode, len(s.response.StatProofs))
	for i := range s.response.StatProofs {
		out[i] = append([]ProofNode(nil), s.response.StatProofs[i]...)
	}
	return out
}

func (s *ScoresStatValidationV2) Response() ScoresStatValidationV2Response {
	return s.response
}

func (s *ScoresStatValidationV2) TargetTS() int64 {
	return s.response.Summary.UpdateStats.MinTimestamp
}

func (s *ScoresStatValidationV2) EpochDay() (uint16, error) {
	return TimestampMSToEpochDay(s.TargetTS())
}

func (s *ScoresStatValidationV2) ToValidationInput() StatValidationInput {
	stats := make([]StatLeafInput, len(s.response.StatsToProve))
	for i := range s.response.StatsToProve {
		stats[i] = StatLeafInput{
			Stat:      s.response.StatsToProve[i],
			StatProof: append([]ProofNode(nil), s.response.StatProofs[i]...),
		}
	}
	return StatValidationInput{
		Ts: s.TargetTS(),
		FixtureSummary: FixtureSummaryInput{
			FixtureID:         s.response.Summary.FixtureID,
			UpdateCount:       s.response.Summary.UpdateStats.UpdateCount,
			MinTimestamp:      s.response.Summary.UpdateStats.MinTimestamp,
			MaxTimestamp:      s.response.Summary.UpdateStats.MaxTimestamp,
			EventsSubTreeRoot: s.response.Summary.EventStatsSubTreeRoot.Bytes(),
		},
		FixtureProof:  append([]ProofNode(nil), s.response.SubTreeProof...),
		MainTreeProof: append([]ProofNode(nil), s.response.MainTreeProof...),
		EventStatRoot: s.response.EventStatRoot.Bytes(),
		Stats:         stats,
	}
}

func (s *ScoresStatValidationV2) LeadingSubset(length int) (StatValidationInput, error) {
	if length == 0 || length > len(s.response.StatsToProve) {
		return StatValidationInput{}, &Error{Kind: ErrValidation, Msg: "V2 payload subset length must be within the proved stat count"}
	}
	input := s.ToValidationInput()
	input.Stats = input.Stats[:length]
	return input, nil
}

func EnsurePositiveSeq(seq int32) error {
	if seq <= 0 {
		return &Error{Kind: ErrInvalidInput, Msg: "score stat validation seq must be greater than zero and must come from a real score record"}
	}
	return nil
}

func TimestampMSToEpochDay(timestampMS int64) (uint16, error) {
	if timestampMS < 0 {
		return 0, &Error{Kind: ErrValidation, Msg: "validation timestamp must not be negative"}
	}
	day := timestampMS / 86_400_000
	if day > 0xffff {
		return 0, &Error{Kind: ErrValidation, Msg: "epoch day does not fit into u16 PDA seed"}
	}
	return uint16(day), nil
}
