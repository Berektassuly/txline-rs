package txline

import (
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"errors"
	"testing"
)

func TestProofDecoding(t *testing.T) {
	bytes := hashBytes(9)
	std := base64.StdEncoding.EncodeToString(bytes[:])
	got, err := DecodeHash32(std)
	if err != nil || got.Bytes() != bytes {
		t.Fatalf("standard base64 decode mismatch: %v", err)
	}
	hexValue := "0x" + hex.EncodeToString(bytes[:])
	got, err = DecodeHash32(hexValue)
	if err != nil || got.Bytes() != bytes {
		t.Fatalf("hex decode mismatch: %v", err)
	}
	var fromArray Hash32
	raw, _ := json.Marshal(bytes[:])
	if err := json.Unmarshal(raw, &fromArray); err != nil || fromArray.Bytes() != bytes {
		t.Fatalf("byte-array decode mismatch: %v", err)
	}
	if _, err := DecodeHash32(base64.StdEncoding.EncodeToString([]byte{1, 2, 3})); !errors.Is(err, ErrProofDecode) {
		t.Fatalf("short proof should fail with ErrProofDecode: %v", err)
	}
}

func TestV2ValidationShapeAndOrderChecks(t *testing.T) {
	valid, err := NewScoresStatValidationV2([]uint32{1001, 1002}, v2ResponseForTest(2))
	if err != nil {
		t.Fatalf("valid response rejected: %v", err)
	}
	if valid.TargetTS() != valid.Response().Summary.UpdateStats.MinTimestamp {
		t.Fatal("TargetTS should use summary minTimestamp")
	}
	if len(valid.ToValidationInput().Stats) != 2 {
		t.Fatal("expected two validation input stats")
	}
	if _, err := NewScoresStatValidationV2([]uint32{1001, 1002, 1003}, v2ResponseForTest(2)); !errors.Is(err, ErrValidation) {
		t.Fatalf("length mismatch should be validation error: %v", err)
	}
	response := v2ResponseForTest(2)
	response.StatsToProve[0], response.StatsToProve[1] = response.StatsToProve[1], response.StatsToProve[0]
	if _, err := NewScoresStatValidationV2([]uint32{1001, 1002}, response); !errors.Is(err, ErrValidation) {
		t.Fatalf("order mismatch should be validation error: %v", err)
	}
	response = v2ResponseForTest(2)
	response.StatProofs = response.StatProofs[:1]
	if _, err := NewScoresStatValidationV2([]uint32{1001, 1002}, response); !errors.Is(err, ErrValidation) {
		t.Fatalf("proof length mismatch should be validation error: %v", err)
	}
}

func TestStrategyBuilderBoundsChecks(t *testing.T) {
	predicate := NewTraderPredicate(0, EqualTo())
	if _, err := NewStrategyBuilder(2).Binary(0, 2, Subtract(), predicate).Build(); !errors.Is(err, ErrValidation) {
		t.Fatalf("out-of-bounds binary predicate should fail: %v", err)
	}
	if _, err := NewStrategyBuilder(2).GeometricTarget(0, 0).Build(); !errors.Is(err, ErrValidation) {
		t.Fatalf("geometric target without distance predicate should fail: %v", err)
	}
	strategy, err := NewStrategyBuilder(3).
		Single(0, NewTraderPredicate(1, GreaterThan())).
		Binary(0, 1, Subtract(), predicate).
		GeometricTarget(2, 4).
		DistancePredicate(NewTraderPredicate(5, LessThan())).
		Build()
	if err != nil {
		t.Fatalf("multi-shape strategy rejected: %v", err)
	}
	if len(strategy.DiscretePredicates) != 2 || len(strategy.GeometricTargets) != 1 {
		t.Fatalf("unexpected strategy shape: %+v", strategy)
	}
}
