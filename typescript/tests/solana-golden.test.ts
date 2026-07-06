import { readFileSync } from "node:fs";
import { AccountRole } from "@solana/kit";
import { describe, expect, it } from "vitest";
import {
  addressFromBytes,
  auditTradeResultInstruction,
  binaryExpression,
  claimBatchLegacyInstruction,
  claimViaResolutionInstruction,
  closeIntentInstruction,
  comparison,
  createIntentInstruction,
  createTradeInstruction,
  executeMatchInstruction,
  refundBatchInstruction,
  settleMatchedTradeInstruction,
  settleTradeInstruction,
  traderPredicate,
  validateFixtureBatchInstruction,
  validateFixtureInstruction,
  validateOddsInstruction,
  validateStatInstruction,
  validateStatV2Instruction,
  type FixtureBatchValidation,
  type FixtureValidation,
  type MarketIntentParams,
  type NDimensionalStrategy,
  type OddsValidation,
  type ProofNode,
  type ScoresStatValidation,
  type StatTermInput,
  type StatValidationInput,
  type TxlineInstruction,
} from "../src/index.js";

describe("validation instruction golden fixtures", () => {
  it("matches Rust Devnet Anchor data bytes", () => {
    const programId = key(200);
    const root = key(201);

    expectData(
      validateStatInstruction(
        programId,
        root,
        scoreValidation(),
        traderPredicate(1, comparison.lessThan()),
        binaryExpression.add(),
      ),
      "validation",
      "validate_stat",
    );

    expectData(
      validateStatV2Instruction(programId, root, statV2Payload(), v2Strategy()),
      "validation",
      "validate_stat_v2",
    );

    expectData(
      validateFixtureInstruction(programId, root, fixtureValidation()),
      "validation",
      "validate_fixture",
    );

    expectData(
      validateFixtureBatchInstruction(programId, root, 3, fixtureBatchValidation()),
      "validation",
      "validate_fixture_batch",
    );

    expectData(
      validateOddsInstruction(programId, root, oddsValidation()),
      "validation",
      "validate_odds",
    );
  });
});

describe("trading instruction golden fixtures", () => {
  it("matches Rust Devnet Anchor data bytes", () => {
    const programId = key(200);

    expectData(
      createIntentInstruction(programId, createIntentAccounts(), createIntentParams()),
      "trading",
      "create_intent",
    );
    expectData(
      createTradeInstruction(programId, createTradeAccounts(), createTradeParams()),
      "trading",
      "create_trade",
    );
    expectData(
      executeMatchInstruction(programId, executeMatchAccounts(), executeMatchParams()),
      "trading",
      "execute_match",
    );
    expectData(
      closeIntentInstruction(programId, closeIntentAccounts()),
      "trading",
      "close_intent",
    );
    expectData(
      settleTradeInstruction(programId, settleTradeAccounts(), settleTradeParams()),
      "trading",
      "settle_trade",
    );
    expectData(
      settleMatchedTradeInstruction(
        programId,
        settleMatchedTradeAccounts(),
        settleMatchedTradeParams(),
      ),
      "trading",
      "settle_matched_trade",
    );
    expectData(
      claimViaResolutionInstruction(
        programId,
        claimViaResolutionAccounts(),
        claimViaResolutionParams(),
      ),
      "trading",
      "claim_via_resolution",
    );
    expectData(
      claimBatchLegacyInstruction(
        programId,
        claimBatchLegacyAccounts(),
        claimBatchLegacyParams(),
      ),
      "trading",
      "claim_batch_legacy",
    );
    expectData(
      refundBatchInstruction(programId, refundBatchAccounts()),
      "trading",
      "refund_batch",
    );
    expectData(
      auditTradeResultInstruction(
        programId,
        auditTradeResultAccounts(),
        auditTradeResultParams(),
      ),
      "trading",
      "audit_trade_result",
    );
  });

  it("matches Rust account order and roles for trading builders", () => {
    const programId = key(200);

    expect(createIntentInstruction(programId, createIntentAccounts(), createIntentParams()).accounts).toEqual([
      meta(1, AccountRole.WRITABLE_SIGNER),
      meta(2, AccountRole.WRITABLE),
      meta(3, AccountRole.WRITABLE),
      meta(4, AccountRole.WRITABLE),
      meta(5, AccountRole.READONLY),
      meta(6, AccountRole.READONLY),
      meta(7, AccountRole.READONLY),
      meta(8, AccountRole.READONLY),
    ]);

    expect(createTradeInstruction(programId, createTradeAccounts(), createTradeParams()).accounts).toEqual([
      meta(11, AccountRole.WRITABLE_SIGNER),
      meta(12, AccountRole.WRITABLE_SIGNER),
      meta(13, AccountRole.WRITABLE_SIGNER),
      meta(14, AccountRole.WRITABLE),
      meta(15, AccountRole.WRITABLE),
      meta(16, AccountRole.WRITABLE),
      meta(17, AccountRole.WRITABLE),
      meta(18, AccountRole.READONLY),
      meta(19, AccountRole.READONLY),
      meta(20, AccountRole.READONLY),
      meta(21, AccountRole.READONLY),
    ]);

    expect(executeMatchInstruction(programId, executeMatchAccounts(), executeMatchParams()).accounts).toEqual([
      meta(31, AccountRole.WRITABLE_SIGNER),
      meta(32, AccountRole.WRITABLE),
      meta(33, AccountRole.WRITABLE),
      meta(34, AccountRole.WRITABLE),
      meta(35, AccountRole.WRITABLE),
      meta(36, AccountRole.WRITABLE),
      meta(37, AccountRole.WRITABLE),
      meta(38, AccountRole.READONLY),
      meta(39, AccountRole.READONLY),
      meta(40, AccountRole.READONLY),
    ]);

    expect(closeIntentInstruction(programId, closeIntentAccounts()).accounts).toEqual([
      meta(51, AccountRole.WRITABLE),
      meta(52, AccountRole.WRITABLE_SIGNER),
      meta(53, AccountRole.WRITABLE),
      meta(54, AccountRole.WRITABLE),
      meta(55, AccountRole.WRITABLE),
      meta(56, AccountRole.READONLY),
      meta(57, AccountRole.READONLY),
      meta(58, AccountRole.READONLY),
    ]);
  });
});

function expectData(
  instruction: TxlineInstruction,
  fixtureSet: "validation" | "trading",
  name: string,
): void {
  expect(Buffer.from(instruction.data).toString("hex")).toBe(
    goldenData(fixtureSet, name),
  );
}

function scoreValidation(): ScoresStatValidation {
  return {
    ts: 1_781_123_456_789,
    statToProve: { key: 1001, value: 2, period: 0 },
    eventStatRoot: hashBytes(20),
    summary: {
      fixtureId: 2_147_483_653,
      updateStats: {
        updateCount: -3,
        minTimestamp: 1_781_123_456_789,
        maxTimestamp: 1_781_123_456_799,
      },
      eventStatsSubTreeRoot: hashBytes(10),
    },
    statProof: [proof(30, true)],
    subTreeProof: [proof(50, false)],
    mainTreeProof: [proof(60, true)],
    statToProve2: { key: 1002, value: -1, period: 1 },
    statProof2: [proof(40, false)],
  };
}

function statV2Payload(): StatValidationInput {
  return {
    ts: 1_781_123_456_789,
    fixtureSummary: {
      fixtureId: 2_147_483_653,
      updateCount: -3,
      minTimestamp: 1_781_123_456_789,
      maxTimestamp: 1_781_123_456_799,
      eventsSubTreeRoot: hashBytes(10),
    },
    fixtureProof: [proof(51, false)],
    mainTreeProof: [proof(61, true)],
    eventStatRoot: hashBytes(22),
    stats: [
      {
        stat: { key: 1001, value: 2, period: 0 },
        statProof: [proof(31, true)],
      },
      {
        stat: { key: 1002, value: -1, period: 1 },
        statProof: [proof(41, false)],
      },
    ],
  };
}

function v2Strategy(): NDimensionalStrategy {
  return {
    geometricTargets: [
      { statIndex: 0, prediction: 0 },
      { statIndex: 1, prediction: 1 },
    ],
    distancePredicate: traderPredicate(2, comparison.lessThan()),
    discretePredicates: [
      {
        single: {
          index: 0,
          predicate: traderPredicate(1, comparison.equalTo()),
        },
      },
      {
        binary: {
          indexA: 0,
          indexB: 1,
          op: binaryExpression.subtract(),
          predicate: traderPredicate(0, comparison.greaterThan()),
        },
      },
    ],
  };
}

function fixtureValidation(): FixtureValidation {
  return {
    snapshot: {
      Ts: 1_781_123_000_000,
      StartTime: 1_781_126_600_000,
      Competition: "Devnet Cup",
      CompetitionId: 7,
      FixtureGroupId: -8,
      Participant1Id: 101,
      Participant1: "Alpha",
      Participant2Id: 202,
      Participant2: "Beta",
      FixtureId: 2_147_483_654,
      Participant1IsHome: true,
    },
    summary: {
      fixtureId: 2_147_483_654,
      competitionId: 7,
      competition: "Devnet Cup",
      updateStats: {
        updateCount: 4,
        minTimestamp: 1_781_123_000_000,
        maxTimestamp: 1_781_123_000_001,
      },
      updateSubTreeRoot: hashBytes(70),
    },
    subTreeProof: [proof(71, false)],
    mainTreeProof: [proof(72, true)],
  };
}

function fixtureBatchValidation(): FixtureBatchValidation {
  return {
    metadata: {
      totalUpdateCount: 5,
      numUniqueFixtures: 2,
      overallBatchStartTs: 1_781_123_000_000,
      overallBatchEndTs: 1_781_123_900_000,
    },
    proof: [proof(80, false), proof(81, true)],
  };
}

function oddsValidation(): OddsValidation {
  return {
    odds: {
      FixtureId: 2_147_483_655,
      MessageId: "msg-1",
      Ts: 1_781_123_456_789,
      Bookmaker: "Book",
      BookmakerId: 9,
      SuperOddsType: "Winner",
      GameState: "PreMatch",
      InRunning: false,
      MarketPeriod: "FT",
      PriceNames: ["Home", "Away"],
      Prices: [120, -125],
      Pct: [],
    },
    summary: {
      fixtureId: 2_147_483_655,
      updateStats: {
        updateCount: 5,
        minTimestamp: 1_781_123_450_000,
        maxTimestamp: 1_781_123_459_999,
      },
      oddsSubTreeRoot: hashBytes(90),
    },
    subTreeProof: [proof(91, false)],
    mainTreeProof: [proof(92, true)],
  };
}

function createIntentAccounts() {
  return {
    maker: key(1),
    orderIntent: key(2),
    intentVault: key(3),
    makerTokenAccount: key(4),
    tokenMint: key(5),
    tokenTreasuryPda: key(6),
    tokenProgram: key(7),
    systemProgram: key(8),
  };
}

function createIntentParams() {
  return {
    intentId: 9001,
    termsHash: hashBytes(100),
    depositAmount: 123_456_789,
    expirationTs: 1_781_129_999_999,
    claimPeriod: 42,
    fixtureId: 2_147_483_653,
  };
}

function createTradeAccounts() {
  return {
    authority: key(11),
    traderA: key(12),
    traderB: key(13),
    traderATokenAccount: key(14),
    traderBTokenAccount: key(15),
    tradeEscrow: key(16),
    escrowVault: key(17),
    stakeTokenMint: key(18),
    tokenTreasuryPda: key(19),
    tokenProgram: key(20),
    systemProgram: key(21),
  };
}

function createTradeParams() {
  return {
    tradeId: 9002,
    stakeA: 111_111,
    stakeB: 222_222,
    tradeTermsHash: hashBytes(110),
  };
}

function executeMatchAccounts() {
  return {
    solver: key(31),
    makerIntent: key(32),
    takerIntent: key(33),
    makerVault: key(34),
    takerVault: key(35),
    matchedTrade: key(36),
    tradeVault: key(37),
    tokenMint: key(38),
    tokenProgram: key(39),
    systemProgram: key(40),
  };
}

function executeMatchParams() {
  return {
    tradeId: 9003,
    makerStake: 333_333,
    takerStake: 444_444,
  };
}

function closeIntentAccounts() {
  return {
    maker: key(51),
    authority: key(52),
    orderIntent: key(53),
    intentVault: key(54),
    makerTokenAccount: key(55),
    tokenMint: key(56),
    tokenProgram: key(57),
    tokenTreasuryPda: key(58),
  };
}

function settleTradeAccounts() {
  return {
    winner: key(71),
    dailyScoresMerkleRoots: key(72),
    tradeEscrow: key(73),
    escrowVault: key(74),
    winnerTokenAccount: key(75),
    tokenMint: key(76),
    tokenTreasuryPda: key(77),
    tokenProgram: key(78),
    systemProgram: key(79),
  };
}

function settleTradeParams() {
  return {
    tradeId: 9004,
    ts: 1_781_123_456_789,
    fixtureSummary,
    fixtureProof: [proof(50, false)],
    mainTreeProof: [proof(60, true)],
    predicate: traderPredicate(1, comparison.lessThan()),
    statA: statA(),
    statB: statB(),
    op: binaryExpression.add(),
  };
}

function settleMatchedTradeAccounts() {
  return {
    winner: key(91),
    dailyScoresMerkleRoots: key(92),
    matchedTrade: key(93),
    tradeVault: key(94),
    winnerTokenAccount: key(95),
    tokenMint: key(96),
    tokenTreasuryPda: key(97),
    tokenProgram: key(98),
    systemProgram: key(99),
  };
}

function settleMatchedTradeParams() {
  return {
    tradeId: 9005,
    ts: 1_781_123_456_790,
    fixtureSummary,
    fixtureProof: [proof(51, false)],
    mainTreeProof: [proof(61, true)],
    statA: statA(),
    statB: statB(),
    terms: marketTerms(),
  };
}

function claimViaResolutionAccounts() {
  return {
    winner: key(111),
    dailyResolutionRoots: key(112),
    matchedTrade: key(113),
    tradeVault: key(114),
    winnerTokenAccount: key(115),
    tokenProgram: key(116),
  };
}

function claimViaResolutionParams() {
  return {
    epochDay: 20_615,
    intervalIndex: 17,
    merkleProof: [proof(70, false), proof(71, true)],
  };
}

function claimBatchLegacyAccounts() {
  return {
    payer: key(121),
    dailyResolutionRoots: key(122),
    tokenMint: key(123),
    tokenProgram: key(124),
    systemProgram: key(125),
  };
}

function claimBatchLegacyParams() {
  return {
    epochDay: 20_616,
    intervalIndex: 18,
    termsHash: hashBytes(120),
    winnerIsMaker: true,
    seq: 941,
    merkleProof: [proof(72, false), proof(73, true)],
  };
}

function refundBatchAccounts() {
  return {
    payer: key(131),
    tokenMint: key(132),
    tokenProgram: key(133),
    systemProgram: key(134),
  };
}

function auditTradeResultAccounts() {
  return {
    payer: key(141),
    dailyScoresMerkleRoots: key(142),
  };
}

function auditTradeResultParams() {
  const terms: MarketIntentParams = {
    ...marketTerms(),
    statBKey: undefined,
    op: undefined,
    negation: true,
  };
  return {
    terms,
    fixtureSummary,
    mainTreeProof: [proof(62, true)],
    fixtureProof: [proof(52, false)],
    statA: statA(),
    statB: undefined,
    ts: 1_781_123_456_791,
  };
}

const fixtureSummary = {
  fixtureId: 2_147_483_653,
  updateCount: -3,
  minTimestamp: 1_781_123_456_789,
  maxTimestamp: 1_781_123_456_799,
  eventsSubTreeRoot: hashBytes(10),
};

function statA(): StatTermInput {
  return {
    statToProve: { key: 1001, value: 2, period: 0 },
    eventStatRoot: hashBytes(20),
    statProof: [proof(30, true)],
  };
}

function statB(): StatTermInput {
  return {
    statToProve: { key: 1002, value: -1, period: 1 },
    eventStatRoot: hashBytes(20),
    statProof: [proof(40, false)],
  };
}

function marketTerms(): MarketIntentParams {
  return {
    fixtureId: 2_147_483_653,
    period: 0,
    statAKey: 1001,
    statBKey: 1002,
    predicate: traderPredicate(1, comparison.greaterThan()),
    op: binaryExpression.subtract(),
    negation: false,
  };
}

function proof(base: number, isRightSibling: boolean): ProofNode {
  return { hash: hashBytes(base), isRightSibling };
}

function key(base: number) {
  return addressFromBytes(new Uint8Array(32).fill(base));
}

function meta(base: number, role: AccountRole) {
  return { address: key(base), role };
}

function hashBytes(base: number): Uint8Array {
  return Uint8Array.from({ length: 32 }, (_value, index) => (base + index) & 0xff);
}

function goldenData(fixtureSet: "validation" | "trading", name: string): string {
  const file =
    fixtureSet === "validation"
      ? "../../crates/txline/tests/fixtures/validation_golden.devnet.json"
      : "../../crates/txline/tests/fixtures/trading_golden.devnet.json";
  const golden = JSON.parse(readFileSync(new URL(file, import.meta.url), "utf8")) as {
    fixtures: { name: string; dataHex: string }[];
  };
  const fixture = golden.fixtures.find((item) => item.name === name);
  if (!fixture) {
    throw new Error(`missing golden fixture ${name}`);
  }
  return fixture.dataHex;
}
