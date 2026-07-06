package txline

import "fmt"

type Comparison uint8

const (
	ComparisonGreaterThan Comparison = iota
	ComparisonLessThan
	ComparisonEqualTo
)

func GreaterThan() Comparison { return ComparisonGreaterThan }
func LessThan() Comparison    { return ComparisonLessThan }
func EqualTo() Comparison     { return ComparisonEqualTo }

type BinaryExpression uint8

const (
	BinaryExpressionAdd BinaryExpression = iota
	BinaryExpressionSubtract
)

func Add() BinaryExpression      { return BinaryExpressionAdd }
func Subtract() BinaryExpression { return BinaryExpressionSubtract }

type TraderPredicate struct {
	Threshold  int32      `json:"threshold"`
	Comparison Comparison `json:"comparison"`
}

func NewTraderPredicate(threshold int32, comparison Comparison) TraderPredicate {
	return TraderPredicate{Threshold: threshold, Comparison: comparison}
}

type StatPredicateKind uint8

const (
	StatPredicateSingle StatPredicateKind = iota
	StatPredicateBinary
)

type StatPredicate struct {
	Kind      StatPredicateKind `json:"kind"`
	Index     uint8             `json:"index,omitempty"`
	IndexA    uint8             `json:"indexA,omitempty"`
	IndexB    uint8             `json:"indexB,omitempty"`
	Op        BinaryExpression  `json:"op,omitempty"`
	Predicate TraderPredicate   `json:"predicate"`
}

type GeometricTarget struct {
	StatIndex  uint8 `json:"statIndex"`
	Prediction int32 `json:"prediction"`
}

type NDimensionalStrategy struct {
	GeometricTargets   []GeometricTarget `json:"geometricTargets"`
	DistancePredicate  *TraderPredicate  `json:"distancePredicate,omitempty"`
	DiscretePredicates []StatPredicate   `json:"discretePredicates"`
}

func NewStrategyBuilder(statCount int) *StrategyBuilder {
	return &StrategyBuilder{
		statCount: statCount,
		strategy: NDimensionalStrategy{
			GeometricTargets:   []GeometricTarget{},
			DiscretePredicates: []StatPredicate{},
		},
	}
}

type StrategyBuilder struct {
	statCount int
	strategy  NDimensionalStrategy
	err       error
}

func (b *StrategyBuilder) Single(index uint8, predicate TraderPredicate) *StrategyBuilder {
	if b.err != nil {
		return b
	}
	if err := ensureStrategyIndex(index, b.statCount); err != nil {
		b.err = err
		return b
	}
	b.strategy.DiscretePredicates = append(b.strategy.DiscretePredicates, StatPredicate{
		Kind:      StatPredicateSingle,
		Index:     index,
		Predicate: predicate,
	})
	return b
}

func (b *StrategyBuilder) Binary(indexA, indexB uint8, op BinaryExpression, predicate TraderPredicate) *StrategyBuilder {
	if b.err != nil {
		return b
	}
	if err := ensureStrategyIndex(indexA, b.statCount); err != nil {
		b.err = err
		return b
	}
	if err := ensureStrategyIndex(indexB, b.statCount); err != nil {
		b.err = err
		return b
	}
	b.strategy.DiscretePredicates = append(b.strategy.DiscretePredicates, StatPredicate{
		Kind:      StatPredicateBinary,
		IndexA:    indexA,
		IndexB:    indexB,
		Op:        op,
		Predicate: predicate,
	})
	return b
}

func (b *StrategyBuilder) GeometricTarget(statIndex uint8, prediction int32) *StrategyBuilder {
	if b.err != nil {
		return b
	}
	if err := ensureStrategyIndex(statIndex, b.statCount); err != nil {
		b.err = err
		return b
	}
	b.strategy.GeometricTargets = append(b.strategy.GeometricTargets, GeometricTarget{
		StatIndex:  statIndex,
		Prediction: prediction,
	})
	return b
}

func (b *StrategyBuilder) DistancePredicate(predicate TraderPredicate) *StrategyBuilder {
	if b.err != nil {
		return b
	}
	b.strategy.DistancePredicate = &predicate
	return b
}

func (b *StrategyBuilder) Build() (NDimensionalStrategy, error) {
	if b.err != nil {
		return NDimensionalStrategy{}, b.err
	}
	if err := b.strategy.ValidateIndices(b.statCount); err != nil {
		return NDimensionalStrategy{}, err
	}
	return b.strategy, nil
}

func (s NDimensionalStrategy) ValidateIndices(statCount int) error {
	for _, target := range s.GeometricTargets {
		if err := ensureStrategyIndex(target.StatIndex, statCount); err != nil {
			return err
		}
	}
	for _, predicate := range s.DiscretePredicates {
		switch predicate.Kind {
		case StatPredicateSingle:
			if err := ensureStrategyIndex(predicate.Index, statCount); err != nil {
				return err
			}
		case StatPredicateBinary:
			if err := ensureStrategyIndex(predicate.IndexA, statCount); err != nil {
				return err
			}
			if err := ensureStrategyIndex(predicate.IndexB, statCount); err != nil {
				return err
			}
		default:
			return &Error{Kind: ErrValidation, Msg: fmt.Sprintf("unknown stat predicate kind %d", predicate.Kind)}
		}
	}
	if len(s.GeometricTargets) > 0 && s.DistancePredicate == nil {
		return &Error{Kind: ErrValidation, Msg: "geometric targets require a distance predicate"}
	}
	return nil
}

func ensureStrategyIndex(index uint8, statCount int) error {
	if int(index) >= statCount {
		return &Error{Kind: ErrValidation, Msg: fmt.Sprintf("strategy index %d is out of bounds for %d requested stat keys", index, statCount)}
	}
	return nil
}
