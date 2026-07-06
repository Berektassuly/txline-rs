import { ValidationPayloadError } from "../errors.js";
import type {
  FixtureBatchValidation,
  FixtureValidation,
  OddsValidation,
} from "../http/models.js";
import {
  fixtureSummaryInput,
  primaryStatTerm,
  secondaryStatTerm,
  timestampMsToEpochDay,
  type ScoresStatValidation,
} from "../validation/legacy.js";
import {
  validateStrategyIndices,
  type BinaryExpression,
  type NDimensionalStrategy,
  type TraderPredicate,
} from "../validation/strategy.js";
import type { StatValidationInput } from "../validation/v2.js";
import {
  encodeBinaryExpression,
  encodeFixture,
  encodeNDimensionalStrategy,
  encodeOdds,
  encodeProofVec,
  encodeScoresBatchSummary,
  encodeStatTerm,
  encodeStatValidationInput,
  encodeTraderPredicate,
  encodeUpdateStatsU32,
  encodeWithDiscriminator,
  hash32Bytes,
  nonnegativeU32,
} from "./codec.js";
import { DevnetPdas } from "./pda.js";
import {
  readonly,
  toAddress,
  type AddressLike,
  type TxlineInstruction,
} from "./types.js";

export const VALIDATE_FIXTURE_DISCRIMINATOR = [
  231, 129, 218, 86, 223, 114, 21, 126,
] as const;
export const VALIDATE_FIXTURE_BATCH_DISCRIMINATOR = [
  85, 223, 204, 7, 4, 87, 157, 1,
] as const;
export const VALIDATE_ODDS_DISCRIMINATOR = [
  192, 19, 91, 138, 104, 100, 212, 86,
] as const;
export const VALIDATE_STAT_DISCRIMINATOR = [
  107, 197, 232, 90, 191, 136, 105, 185,
] as const;
export const VALIDATE_STAT_V2_DISCRIMINATOR = [
  208, 215, 194, 214, 241, 71, 246, 178,
] as const;

export const DEFAULT_VALIDATION_COMPUTE_UNITS = 1_400_000;

export function validateStatInstruction(
  programId: AddressLike,
  dailyScoresMerkleRoots: AddressLike,
  validation: ScoresStatValidation,
  predicate: TraderPredicate,
  op?: BinaryExpression,
): TxlineInstruction {
  const statA = primaryStatTerm(validation);
  const statB = secondaryStatTerm(validation);
  const targetTs = validation.summary.updateStats.minTimestamp;
  return {
    programAddress: toAddress(programId),
    accounts: [readonly(dailyScoresMerkleRoots)],
    data: encodeWithDiscriminator(VALIDATE_STAT_DISCRIMINATOR, (writer) => {
      writer.putI64(targetTs);
      encodeScoresBatchSummary(writer, fixtureSummaryInput(validation));
      encodeProofVec(writer, validation.subTreeProof ?? []);
      encodeProofVec(writer, validation.mainTreeProof ?? []);
      encodeTraderPredicate(writer, predicate);
      encodeStatTerm(writer, statA);
      writer.putOption(statB, encodeStatTerm);
      writer.putOption(op, encodeBinaryExpression);
    }),
  };
}

export async function devnetValidateStatInstruction(
  programId: AddressLike,
  validation: ScoresStatValidation,
  predicate: TraderPredicate,
  op?: BinaryExpression,
): Promise<TxlineInstruction> {
  const pdas = new DevnetPdas();
  const epochDay = timestampMsToEpochDay(validation.summary.updateStats.minTimestamp);
  const root = (await pdas.dailyScoresRoots(epochDay)).address;
  return validateStatInstruction(programId, root, validation, predicate, op);
}

export function validateStatV2Instruction(
  programId: AddressLike,
  dailyScoresMerkleRoots: AddressLike,
  payload: StatValidationInput,
  strategy: NDimensionalStrategy,
): TxlineInstruction {
  validateStrategyIndices(strategy, payload.stats.length);
  return {
    programAddress: toAddress(programId),
    accounts: [readonly(dailyScoresMerkleRoots)],
    data: encodeWithDiscriminator(VALIDATE_STAT_V2_DISCRIMINATOR, (writer) => {
      encodeStatValidationInput(writer, payload);
      encodeNDimensionalStrategy(writer, strategy);
    }),
  };
}

export async function devnetValidateStatV2Instruction(
  programId: AddressLike,
  payload: StatValidationInput,
  strategy: NDimensionalStrategy,
): Promise<TxlineInstruction> {
  const pdas = new DevnetPdas();
  const epochDay = timestampMsToEpochDay(payload.ts);
  const root = (await pdas.dailyScoresRoots(epochDay)).address;
  return validateStatV2Instruction(programId, root, payload, strategy);
}

export function validateFixtureInstruction(
  programId: AddressLike,
  tenDailyFixturesRoots: AddressLike,
  validation: FixtureValidation,
): TxlineInstruction {
  return {
    programAddress: toAddress(programId),
    accounts: [readonly(tenDailyFixturesRoots)],
    data: encodeWithDiscriminator(VALIDATE_FIXTURE_DISCRIMINATOR, (writer) => {
      encodeFixture(writer, validation.snapshot);
      writer.putI64(validation.summary.fixtureId);
      writer.putI32(validation.summary.competitionId);
      writer.putString(validation.summary.competition);
      encodeUpdateStatsU32(writer, validation.summary.updateStats);
      writer.writeBytes(hash32Bytes(validation.summary.updateSubTreeRoot));
      encodeProofVec(writer, validation.subTreeProof ?? []);
      encodeProofVec(writer, validation.mainTreeProof ?? []);
    }),
  };
}

export async function devnetValidateFixtureInstruction(
  programId: AddressLike,
  validation: FixtureValidation,
): Promise<TxlineInstruction> {
  const pdas = new DevnetPdas();
  const epochDay = timestampMsToEpochDay(
    validation.summary.updateStats.minTimestamp,
  );
  const root = (await pdas.tenDailyFixturesRoots(epochDay)).address;
  return validateFixtureInstruction(programId, root, validation);
}

export function validateFixtureBatchInstruction(
  programId: AddressLike,
  tenDailyFixturesRoots: AddressLike,
  index: number,
  validation: FixtureBatchValidation,
): TxlineInstruction {
  if (!Number.isInteger(index) || index < 0 || index > 255) {
    throw new ValidationPayloadError("fixture batch index must fit in u8");
  }
  return {
    programAddress: toAddress(programId),
    accounts: [readonly(tenDailyFixturesRoots)],
    data: encodeWithDiscriminator(
      VALIDATE_FIXTURE_BATCH_DISCRIMINATOR,
      (writer) => {
        writer.putU8(index);
        writer.putI32(validation.metadata.totalUpdateCount);
        writer.putI32(validation.metadata.numUniqueFixtures);
        writer.putI64(validation.metadata.overallBatchStartTs);
        writer.putI64(validation.metadata.overallBatchEndTs);
        encodeProofVec(writer, validation.proof ?? []);
      },
    ),
  };
}

export async function devnetValidateFixtureBatchInstruction(
  programId: AddressLike,
  epochDay: number,
  index: number,
  validation: FixtureBatchValidation,
): Promise<TxlineInstruction> {
  const pdas = new DevnetPdas();
  const root = (await pdas.tenDailyFixturesRoots(epochDay)).address;
  return validateFixtureBatchInstruction(programId, root, index, validation);
}

export function validateOddsInstruction(
  programId: AddressLike,
  dailyOddsMerkleRoots: AddressLike,
  validation: OddsValidation,
): TxlineInstruction {
  return {
    programAddress: toAddress(programId),
    accounts: [readonly(dailyOddsMerkleRoots)],
    data: encodeWithDiscriminator(VALIDATE_ODDS_DISCRIMINATOR, (writer) => {
      writer.putI64(validation.odds.Ts);
      encodeOdds(writer, validation.odds);
      writer.putI64(validation.summary.fixtureId);
      encodeUpdateStatsU32(writer, validation.summary.updateStats);
      writer.writeBytes(hash32Bytes(validation.summary.oddsSubTreeRoot));
      encodeProofVec(writer, validation.subTreeProof ?? []);
      encodeProofVec(writer, validation.mainTreeProof ?? []);
    }),
  };
}

export async function devnetValidateOddsInstruction(
  programId: AddressLike,
  validation: OddsValidation,
): Promise<TxlineInstruction> {
  const pdas = new DevnetPdas();
  const epochDay = timestampMsToEpochDay(
    validation.summary.updateStats.minTimestamp,
  );
  const root = (await pdas.dailyOddsMerkleRoots(epochDay)).address;
  return validateOddsInstruction(programId, root, validation);
}

export function computeUnitLimitInstruction(units: number): TxlineInstruction {
  return {
    programAddress: toAddress("ComputeBudget111111111111111111111111111111"),
    accounts: [],
    data: encodeWithDiscriminator([2], (writer) => writer.putU32(units)),
  };
}

export function checkedUpdateCount(updateCount: number): number {
  return nonnegativeU32(updateCount, "updateCount");
}
