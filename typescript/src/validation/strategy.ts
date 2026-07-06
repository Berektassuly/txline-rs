import { ValidationPayloadError } from "../errors.js";

export type Comparison =
  | { readonly greaterThan: Record<string, never> }
  | { readonly lessThan: Record<string, never> }
  | { readonly equalTo: Record<string, never> };

export type BinaryExpression =
  | { readonly add: Record<string, never> }
  | { readonly subtract: Record<string, never> };

export interface TraderPredicate {
  readonly threshold: number;
  readonly comparison: Comparison;
}

export type StatPredicate =
  | {
      readonly single: {
        readonly index: number;
        readonly predicate: TraderPredicate;
      };
    }
  | {
      readonly binary: {
        readonly indexA: number;
        readonly indexB: number;
        readonly op: BinaryExpression;
        readonly predicate: TraderPredicate;
      };
    };

export interface GeometricTarget {
  readonly statIndex: number;
  readonly prediction: number;
}

export interface NDimensionalStrategy {
  readonly geometricTargets: readonly GeometricTarget[];
  readonly distancePredicate?: TraderPredicate;
  readonly discretePredicates: readonly StatPredicate[];
}

export const comparison = Object.freeze({
  greaterThan: (): Comparison => ({ greaterThan: {} }),
  lessThan: (): Comparison => ({ lessThan: {} }),
  equalTo: (): Comparison => ({ equalTo: {} }),
});

export const binaryExpression = Object.freeze({
  add: (): BinaryExpression => ({ add: {} }),
  subtract: (): BinaryExpression => ({ subtract: {} }),
});

export function traderPredicate(
  threshold: number,
  comparisonValue: Comparison,
): TraderPredicate {
  return { threshold, comparison: comparisonValue };
}

export class StrategyBuilder {
  readonly #statCount: number;
  readonly #geometricTargets: GeometricTarget[] = [];
  readonly #discretePredicates: StatPredicate[] = [];
  #distancePredicate: TraderPredicate | undefined;

  constructor(statCount: number) {
    if (!Number.isInteger(statCount) || statCount < 0) {
      throw new ValidationPayloadError("stat count must be a nonnegative integer");
    }
    this.#statCount = statCount;
  }

  single(index: number, predicate: TraderPredicate): this {
    ensureIndex(index, this.#statCount);
    this.#discretePredicates.push({ single: { index, predicate } });
    return this;
  }

  binary(
    indexA: number,
    indexB: number,
    op: BinaryExpression,
    predicate: TraderPredicate,
  ): this {
    ensureIndex(indexA, this.#statCount);
    ensureIndex(indexB, this.#statCount);
    this.#discretePredicates.push({
      binary: { indexA, indexB, op, predicate },
    });
    return this;
  }

  geometricTarget(statIndex: number, prediction: number): this {
    ensureIndex(statIndex, this.#statCount);
    this.#geometricTargets.push({ statIndex, prediction });
    return this;
  }

  distancePredicate(predicate: TraderPredicate): this {
    this.#distancePredicate = predicate;
    return this;
  }

  build(): NDimensionalStrategy {
    const strategy: NDimensionalStrategy = {
      geometricTargets: [...this.#geometricTargets],
      ...(this.#distancePredicate
        ? { distancePredicate: this.#distancePredicate }
        : {}),
      discretePredicates: [...this.#discretePredicates],
    };
    validateStrategyIndices(strategy, this.#statCount);
    return strategy;
  }
}

export function strategyBuilder(statCount: number): StrategyBuilder {
  return new StrategyBuilder(statCount);
}

export function validateStrategyIndices(
  strategy: NDimensionalStrategy,
  statCount: number,
): void {
  for (const target of strategy.geometricTargets) {
    ensureIndex(target.statIndex, statCount);
  }
  for (const predicate of strategy.discretePredicates) {
    if ("single" in predicate) {
      ensureIndex(predicate.single.index, statCount);
    } else {
      ensureIndex(predicate.binary.indexA, statCount);
      ensureIndex(predicate.binary.indexB, statCount);
    }
  }
  if (strategy.geometricTargets.length > 0 && !strategy.distancePredicate) {
    throw new ValidationPayloadError(
      "geometric targets require a distance predicate",
    );
  }
}

function ensureIndex(index: number, statCount: number): void {
  if (!Number.isInteger(index) || index < 0 || index >= statCount) {
    throw new ValidationPayloadError(
      `strategy index ${index} is out of bounds for ${statCount} requested stat keys`,
    );
  }
}
