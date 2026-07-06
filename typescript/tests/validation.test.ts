import { describe, expect, it } from "vitest";
import {
  ScoresStatValidationV2,
  binaryExpression,
  comparison,
  decodeHash32,
  strategyBuilder,
  traderPredicate,
} from "../src/index.js";

describe("proof decoding", () => {
  it("accepts base64, hex, prefixed hex, and byte arrays", () => {
    const expected = bytes(1);
    const base64 = btoa(String.fromCharCode(...expected));
    const hex = Buffer.from(expected).toString("hex");

    expect([...decodeHash32(base64)]).toEqual([...expected]);
    expect([...decodeHash32(hex)]).toEqual([...expected]);
    expect([...decodeHash32(`0x${hex}`)]).toEqual([...expected]);
    expect([...decodeHash32(expected)]).toEqual([...expected]);
  });

  it("rejects invalid proof lengths", () => {
    expect(() => decodeHash32([1, 2, 3])).toThrow(/expected 32 bytes/u);
  });
});

describe("V2 stat validation", () => {
  it("checks requested stat key order and response lengths", () => {
    const validation = ScoresStatValidationV2.fromResponse(
      [1001, 1002],
      responseWith(2),
    );

    expect(validation.requestedStatKeys()).toEqual([1001, 1002]);
    expect(validation.toValidationInput().stats).toHaveLength(2);
  });

  it("rejects length and order mismatches", () => {
    expect(() =>
      ScoresStatValidationV2.fromResponse([1001, 1002, 1003], responseWith(2)),
    ).toThrow(/statsToProve length/u);

    const response = responseWith(2);
    response.statsToProve = [...response.statsToProve].reverse();
    expect(() => ScoresStatValidationV2.fromResponse([1001, 1002], response)).toThrow(
      /statsToProve\[0\]\.key/u,
    );
  });

  it("uses summary min timestamp for instruction payloads", () => {
    const validation = ScoresStatValidationV2.fromResponse([1001], {
      ...responseWith(1),
      ts: 172_900_000,
      summary: {
        ...responseWith(1).summary,
        updateStats: {
          updateCount: 1,
          minTimestamp: 86_400_000,
          maxTimestamp: 86_400_001,
        },
      },
    });

    expect(validation.targetTs()).toBe(86_400_000);
    expect(validation.toValidationInput().ts).toBe(86_400_000);
  });
});

describe("strategy builder", () => {
  it("builds single, binary, geometric, and distance predicates", () => {
    const eq = traderPredicate(0, comparison.equalTo());
    const gt = traderPredicate(1, comparison.greaterThan());
    const lt = traderPredicate(2, comparison.lessThan());

    const strategy = strategyBuilder(2)
      .single(0, gt)
      .binary(0, 1, binaryExpression.subtract(), eq)
      .geometricTarget(0, 0)
      .geometricTarget(1, 1)
      .distancePredicate(lt)
      .build();

    expect(strategy.discretePredicates).toHaveLength(2);
    expect(strategy.geometricTargets).toHaveLength(2);
  });

  it("checks index bounds before instruction construction", () => {
    expect(() =>
      strategyBuilder(2).binary(
        0,
        2,
        binaryExpression.subtract(),
        traderPredicate(0, comparison.equalTo()),
      ),
    ).toThrow(/out of bounds/u);
  });
});

function responseWith(count: number) {
  return {
    ts: 86_400_000,
    statsToProve: Array.from({ length: count }, (_value, index) => ({
      key: 1001 + index,
      value: index,
      period: 0,
    })),
    eventStatRoot: bytes(9),
    summary: {
      fixtureId: 1,
      updateStats: {
        updateCount: 1,
        minTimestamp: 86_400_000,
        maxTimestamp: 86_400_001,
      },
      eventStatsSubTreeRoot: bytes(8),
    },
    statProofs: Array.from({ length: count }, () => []),
    subTreeProof: [],
    mainTreeProof: [],
  };
}

function bytes(base: number): number[] {
  return Array.from({ length: 32 }, (_value, index) => (base + index) & 0xff);
}
