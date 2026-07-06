package txline

func ptrInt32(value int32) *int32 { return &value }

func ptrString(value string) *string { return &value }

func ptrUint32(value uint32) *uint32 { return &value }

func hash32(base byte) Hash32 {
	bytes := hashBytes(base)
	hash, err := NewHash32(bytes[:])
	if err != nil {
		panic(err)
	}
	return hash
}

func hashBytes(base byte) [32]byte {
	var out [32]byte
	for i := range out {
		out[i] = base + byte(i)
	}
	return out
}

func proof(base byte, right bool) ProofNode {
	return ProofNode{Hash: hash32(base), IsRightSibling: right}
}

func v2ResponseForTest(count int) ScoresStatValidationV2Response {
	hash := hash32(9)
	stats := make([]ScoreStat, count)
	proofs := make([][]ProofNode, count)
	for i := 0; i < count; i++ {
		stats[i] = ScoreStat{Key: 1001 + uint32(i), Value: int32(i), Period: 0}
		proofs[i] = []ProofNode{}
	}
	return ScoresStatValidationV2Response{
		Ts:            1_781_200_000_000,
		StatsToProve:  stats,
		EventStatRoot: hash,
		Summary: ScoresBatchSummary{
			FixtureID: 1,
			UpdateStats: UpdateStats{
				UpdateCount:  1,
				MinTimestamp: 1_781_123_456_789,
				MaxTimestamp: 1_781_200_000_000,
			},
			EventStatsSubTreeRoot: hash,
		},
		StatProofs:    proofs,
		SubTreeProof:  []ProofNode{},
		MainTreeProof: []ProofNode{},
	}
}
