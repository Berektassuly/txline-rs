import { ValidationPayloadError } from "../errors.js";
import type { UpdateStats } from "../http/models.js";
import { decodeHash32, type Hash32Like, type ProofNode } from "./proof.js";

export interface ScoreStat {
  readonly key: number;
  readonly value: number;
  readonly period: number;
}

export interface ScoresBatchSummary {
  fixtureId: number;
  updateStats: UpdateStats;
  eventStatsSubTreeRoot: Hash32Like;
}

export interface ScoresStatValidation {
  readonly ts: number;
  readonly statToProve: ScoreStat;
  readonly eventStatRoot: Hash32Like;
  readonly summary: ScoresBatchSummary;
  readonly statProof?: ProofNode[];
  readonly subTreeProof?: ProofNode[];
  readonly mainTreeProof?: ProofNode[];
  readonly statToProve2?: ScoreStat;
  readonly statProof2?: ProofNode[];
}

export interface FixtureSummaryInput {
  readonly fixtureId: number;
  readonly updateCount: number;
  readonly minTimestamp: number;
  readonly maxTimestamp: number;
  readonly eventsSubTreeRoot: Uint8Array;
}

export interface StatTermInput {
  readonly statToProve: ScoreStat;
  readonly eventStatRoot: Uint8Array;
  readonly statProof: readonly ProofNode[];
}

export function fixtureSummaryInput(
  validation: ScoresStatValidation,
): FixtureSummaryInput {
  return {
    fixtureId: validation.summary.fixtureId,
    updateCount: validation.summary.updateStats.updateCount,
    minTimestamp: validation.summary.updateStats.minTimestamp,
    maxTimestamp: validation.summary.updateStats.maxTimestamp,
    eventsSubTreeRoot: decodeHash32(validation.summary.eventStatsSubTreeRoot),
  };
}

export function primaryStatTerm(validation: ScoresStatValidation): StatTermInput {
  return {
    statToProve: validation.statToProve,
    eventStatRoot: decodeHash32(validation.eventStatRoot),
    statProof: validation.statProof ?? [],
  };
}

export function secondaryStatTerm(
  validation: ScoresStatValidation,
): StatTermInput | undefined {
  if (validation.statToProve2 && validation.statProof2) {
    return {
      statToProve: validation.statToProve2,
      eventStatRoot: decodeHash32(validation.eventStatRoot),
      statProof: validation.statProof2,
    };
  }
  if (!validation.statToProve2 && !validation.statProof2) {
    return undefined;
  }
  throw new ValidationPayloadError(
    "legacy response contains only one of statToProve2/statProof2",
  );
}

export function timestampMsToEpochDay(timestampMs: number): number {
  if (timestampMs < 0) {
    throw new ValidationPayloadError("validation timestamp must not be negative");
  }
  const epochDay = Math.trunc(timestampMs / 86_400_000);
  if (epochDay > 0xffff) {
    throw new ValidationPayloadError("epoch day does not fit into u16 PDA seed");
  }
  return epochDay;
}
