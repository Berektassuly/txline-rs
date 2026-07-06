import { InvalidInputError, ValidationPayloadError } from "../errors.js";
import {
  fixtureSummaryInput,
  timestampMsToEpochDay,
  type FixtureSummaryInput,
  type ScoreStat,
  type ScoresBatchSummary,
} from "./legacy.js";
import { decodeHash32, type Hash32Like, type ProofNode } from "./proof.js";

export interface ScoresStatValidationV2Response {
  readonly ts: number;
  readonly statsToProve?: ScoreStat[];
  readonly eventStatRoot: Hash32Like;
  readonly summary: ScoresBatchSummary;
  readonly statProofs?: ProofNode[][];
  readonly subTreeProof?: ProofNode[];
  readonly mainTreeProof?: ProofNode[];
}

export interface StatLeafInput {
  readonly stat: ScoreStat;
  readonly statProof: readonly ProofNode[];
}

export interface StatValidationInput {
  readonly ts: number;
  readonly fixtureSummary: FixtureSummaryInput;
  readonly fixtureProof: readonly ProofNode[];
  readonly mainTreeProof: readonly ProofNode[];
  readonly eventStatRoot: Uint8Array;
  readonly stats: readonly StatLeafInput[];
}

export class ScoresStatValidationV2 {
  readonly #requestedStatKeys: readonly number[];
  readonly #response: ScoresStatValidationV2Response;
  readonly #statsToProve: readonly ScoreStat[];
  readonly #statProofs: readonly ProofNode[][];

  private constructor(
    requestedStatKeys: readonly number[],
    response: ScoresStatValidationV2Response,
    statsToProve: readonly ScoreStat[],
    statProofs: readonly ProofNode[][],
  ) {
    this.#requestedStatKeys = requestedStatKeys;
    this.#response = response;
    this.#statsToProve = statsToProve;
    this.#statProofs = statProofs;
  }

  static fromResponse(
    requestedStatKeys: readonly number[],
    response: ScoresStatValidationV2Response,
  ): ScoresStatValidationV2 {
    if (requestedStatKeys.length === 0) {
      throw new InvalidInputError(
        "V2 stat validation requires at least one stat key",
      );
    }
    const statsToProve = response.statsToProve ?? [];
    if (statsToProve.length !== requestedStatKeys.length) {
      throw new ValidationPayloadError(
        `statsToProve length ${statsToProve.length} does not match requested statKeys length ${requestedStatKeys.length}`,
      );
    }
    statsToProve.forEach((stat, index) => {
      const requested = requestedStatKeys[index];
      if (stat.key !== requested) {
        throw new ValidationPayloadError(
          `statsToProve[${index}].key ${stat.key} does not match requested statKeys[${index}] ${requested}`,
        );
      }
    });
    const statProofs = response.statProofs ?? [];
    if (statProofs.length !== statsToProve.length) {
      throw new ValidationPayloadError(
        `statProofs length ${statProofs.length} does not match statsToProve length ${statsToProve.length}`,
      );
    }
    return new ScoresStatValidationV2(
      [...requestedStatKeys],
      response,
      statsToProve,
      statProofs,
    );
  }

  requestedStatKeys(): readonly number[] {
    return this.#requestedStatKeys;
  }

  statsToProve(): readonly ScoreStat[] {
    return this.#statsToProve;
  }

  statProofs(): readonly ProofNode[][] {
    return this.#statProofs;
  }

  response(): ScoresStatValidationV2Response {
    return this.#response;
  }

  targetTs(): number {
    return this.#response.summary.updateStats.minTimestamp;
  }

  epochDay(): number {
    return timestampMsToEpochDay(this.targetTs());
  }

  toValidationInput(): StatValidationInput {
    const pseudoLegacy = {
      ts: this.#response.ts,
      statToProve: this.#statsToProve[0] ?? { key: 0, value: 0, period: 0 },
      eventStatRoot: this.#response.eventStatRoot,
      summary: this.#response.summary,
    };
    return {
      ts: this.targetTs(),
      fixtureSummary: fixtureSummaryInput(pseudoLegacy),
      fixtureProof: this.#response.subTreeProof ?? [],
      mainTreeProof: this.#response.mainTreeProof ?? [],
      eventStatRoot: decodeHash32(this.#response.eventStatRoot),
      stats: this.#statsToProve.map((stat, index) => ({
        stat,
        statProof: this.#statProofs[index] ?? [],
      })),
    };
  }

  leadingSubset(length: number): StatValidationInput {
    if (
      !Number.isInteger(length) ||
      length <= 0 ||
      length > this.#statsToProve.length
    ) {
      throw new ValidationPayloadError(
        "V2 payload subset length must be within the proved stat count",
      );
    }
    const input = this.toValidationInput();
    return {
      ...input,
      stats: input.stats.slice(0, length),
    };
  }
}
