import { ValidationPayloadError } from "../errors.js";
import type { Fixture, OddsPayload, UpdateStats } from "../http/models.js";
import {
  decodeHash32,
  normalizeProofNode,
  type ProofNode,
} from "../validation/proof.js";
import type {
  BinaryExpression,
  Comparison,
  NDimensionalStrategy,
  StatPredicate,
  TraderPredicate,
} from "../validation/strategy.js";
import type {
  FixtureSummaryInput,
  ScoreStat,
  StatTermInput,
} from "../validation/legacy.js";
import type { StatLeafInput, StatValidationInput } from "../validation/v2.js";

export class ByteWriter {
  readonly #bytes: number[] = [];

  writeBytes(bytes: Uint8Array | readonly number[]): void {
    for (const byte of bytes) {
      if (!Number.isInteger(byte) || byte < 0 || byte > 255) {
        throw new ValidationPayloadError("byte value must be 0..=255");
      }
      this.#bytes.push(byte);
    }
  }

  putBool(value: boolean): void {
    this.putU8(value ? 1 : 0);
  }

  putU8(value: number): void {
    assertIntegerRange(value, 0, 0xff, "u8");
    this.#bytes.push(value);
  }

  putU16(value: number): void {
    assertIntegerRange(value, 0, 0xffff, "u16");
    this.#bytes.push(value & 0xff, (value >> 8) & 0xff);
  }

  putU32(value: number): void {
    assertIntegerRange(value, 0, 0xffff_ffff, "u32");
    const bytes = new Uint8Array(4);
    new DataView(bytes.buffer).setUint32(0, value, true);
    this.writeBytes(bytes);
  }

  putU64(value: number | bigint): void {
    const bigintValue = assertBigIntRange(value, 0n, (1n << 64n) - 1n, "u64");
    const bytes = new Uint8Array(8);
    new DataView(bytes.buffer).setBigUint64(0, bigintValue, true);
    this.writeBytes(bytes);
  }

  putI32(value: number): void {
    assertIntegerRange(value, -0x8000_0000, 0x7fff_ffff, "i32");
    const bytes = new Uint8Array(4);
    new DataView(bytes.buffer).setInt32(0, value, true);
    this.writeBytes(bytes);
  }

  putI64(value: number | bigint): void {
    const bigintValue = assertBigIntRange(
      value,
      -(1n << 63n),
      (1n << 63n) - 1n,
      "i64",
    );
    const bytes = new Uint8Array(8);
    new DataView(bytes.buffer).setBigInt64(0, bigintValue, true);
    this.writeBytes(bytes);
  }

  putString(value: string): void {
    const bytes = new TextEncoder().encode(value);
    this.putVecLength(bytes.length);
    this.writeBytes(bytes);
  }

  putOption<T>(value: T | undefined, encode: (writer: ByteWriter, value: T) => void): void {
    if (value === undefined) {
      this.putU8(0);
      return;
    }
    this.putU8(1);
    encode(this, value);
  }

  putVec<T>(values: readonly T[], encode: (writer: ByteWriter, value: T) => void): void {
    this.putVecLength(values.length);
    for (const value of values) {
      encode(this, value);
    }
  }

  toBytes(): Uint8Array {
    return Uint8Array.from(this.#bytes);
  }

  private putVecLength(length: number): void {
    this.putU32(length);
  }
}

export function encodeWithDiscriminator(
  discriminator: readonly number[],
  encode?: (writer: ByteWriter) => void,
): Uint8Array {
  const writer = new ByteWriter();
  writer.writeBytes(discriminator);
  encode?.(writer);
  return writer.toBytes();
}

export function encodeScoreStat(writer: ByteWriter, stat: ScoreStat): void {
  writer.putU32(stat.key);
  writer.putI32(stat.value);
  writer.putI32(stat.period);
}

export function encodeProofVec(writer: ByteWriter, proof: readonly ProofNode[] = []): void {
  writer.putVec(proof, (out, node) => {
    const normalized = normalizeProofNode(node);
    out.writeBytes(normalized.hash);
    out.putBool(normalized.isRightSibling);
  });
}

export function encodeTraderPredicate(
  writer: ByteWriter,
  predicate: TraderPredicate,
): void {
  writer.putI32(predicate.threshold);
  encodeComparison(writer, predicate.comparison);
}

export function encodeComparison(writer: ByteWriter, comparison: Comparison): void {
  if ("greaterThan" in comparison) {
    writer.putU8(0);
  } else if ("lessThan" in comparison) {
    writer.putU8(1);
  } else {
    writer.putU8(2);
  }
}

export function encodeBinaryExpression(
  writer: ByteWriter,
  op: BinaryExpression,
): void {
  writer.putU8("add" in op ? 0 : 1);
}

export function encodeScoresBatchSummary(
  writer: ByteWriter,
  summary: FixtureSummaryInput,
): void {
  writer.putI64(summary.fixtureId);
  writer.putI32(summary.updateCount);
  writer.putI64(summary.minTimestamp);
  writer.putI64(summary.maxTimestamp);
  writer.writeBytes(summary.eventsSubTreeRoot);
}

export function encodeStatTerm(writer: ByteWriter, term: StatTermInput): void {
  encodeScoreStat(writer, term.statToProve);
  writer.writeBytes(term.eventStatRoot);
  encodeProofVec(writer, term.statProof);
}

export function encodeStatValidationInput(
  writer: ByteWriter,
  input: StatValidationInput,
): void {
  writer.putI64(input.ts);
  encodeScoresBatchSummary(writer, input.fixtureSummary);
  encodeProofVec(writer, input.fixtureProof);
  encodeProofVec(writer, input.mainTreeProof);
  writer.writeBytes(input.eventStatRoot);
  writer.putVec(input.stats, encodeStatLeaf);
}

export function encodeStatLeaf(writer: ByteWriter, leaf: StatLeafInput): void {
  encodeScoreStat(writer, leaf.stat);
  encodeProofVec(writer, leaf.statProof);
}

export function encodeNDimensionalStrategy(
  writer: ByteWriter,
  strategy: NDimensionalStrategy,
): void {
  writer.putVec(strategy.geometricTargets, (out, target) => {
    out.putU8(target.statIndex);
    out.putI32(target.prediction);
  });
  writer.putOption(strategy.distancePredicate, encodeTraderPredicate);
  writer.putVec(strategy.discretePredicates, encodeStatPredicate);
}

export function encodeStatPredicate(
  writer: ByteWriter,
  predicate: StatPredicate,
): void {
  if ("single" in predicate) {
    writer.putU8(0);
    writer.putU8(predicate.single.index);
    encodeTraderPredicate(writer, predicate.single.predicate);
  } else {
    writer.putU8(1);
    writer.putU8(predicate.binary.indexA);
    writer.putU8(predicate.binary.indexB);
    encodeBinaryExpression(writer, predicate.binary.op);
    encodeTraderPredicate(writer, predicate.binary.predicate);
  }
}

export function encodeFixture(writer: ByteWriter, fixture: Fixture): void {
  writer.putI64(fixture.Ts);
  writer.putI64(fixture.StartTime);
  writer.putString(fixture.Competition);
  writer.putI32(fixture.CompetitionId);
  writer.putI32(fixture.FixtureGroupId);
  writer.putI32(fixture.Participant1Id);
  writer.putString(fixture.Participant1);
  writer.putI32(fixture.Participant2Id);
  writer.putString(fixture.Participant2);
  writer.putI64(fixture.FixtureId);
  writer.putBool(fixture.Participant1IsHome);
}

export function encodeUpdateStatsU32(writer: ByteWriter, updateStats: UpdateStats): void {
  writer.putU32(nonnegativeU32(updateStats.updateCount, "updateCount"));
  writer.putI64(updateStats.minTimestamp);
  writer.putI64(updateStats.maxTimestamp);
}

export function encodeOdds(writer: ByteWriter, odds: OddsPayload): void {
  writer.putI64(odds.FixtureId);
  writer.putString(odds.MessageId);
  writer.putI64(odds.Ts);
  writer.putString(odds.Bookmaker);
  writer.putI32(odds.BookmakerId);
  writer.putString(odds.SuperOddsType);
  writer.putOption(odds.GameState, (out, value) => out.putString(value));
  writer.putBool(odds.InRunning);
  writer.putOption(odds.MarketParameters, (out, value) => out.putString(value));
  writer.putOption(odds.MarketPeriod, (out, value) => out.putString(value));
  writer.putVec(odds.PriceNames ?? [], (out, value) => out.putString(value));
  writer.putVec(odds.Prices ?? [], (out, value) => out.putI32(value));
}

export function hash32Bytes(value: string | readonly number[] | Uint8Array): Uint8Array {
  return decodeHash32(value);
}

export function nonnegativeU32(value: number, name: string): number {
  if (!Number.isInteger(value) || value < 0) {
    throw new ValidationPayloadError(
      `${name} must be nonnegative to match the Devnet IDL u32 field`,
    );
  }
  return value;
}

function assertIntegerRange(
  value: number,
  min: number,
  max: number,
  name: string,
): void {
  if (!Number.isSafeInteger(value) || value < min || value > max) {
    throw new ValidationPayloadError(`${name} value ${value} is out of range`);
  }
}

function assertBigIntRange(
  value: number | bigint,
  min: bigint,
  max: bigint,
  name: string,
): bigint {
  const bigintValue = typeof value === "bigint" ? value : BigInt(value);
  if (bigintValue < min || bigintValue > max) {
    throw new ValidationPayloadError(`${name} value ${value} is out of range`);
  }
  return bigintValue;
}
