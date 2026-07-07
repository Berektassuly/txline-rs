import { describe, expect, it } from "vitest";
import {
  addressFromBytes,
  auditTradeResultInstruction,
  auditTradeResultParamsFromV2,
  auditTradeResultPlan,
  binaryExpression,
  claimBatchLegacyInstruction,
  claimBatchLegacyPlan,
  claimViaResolutionInstruction,
  claimViaResolutionPlan,
  closeIntentInstruction,
  closeIntentPlan,
  comparison,
  createIntentInstruction,
  createIntentPlan,
  createTradeInstruction,
  createTradePlan,
  defaultSoccerFinalOutcomeConfig,
  ensureTermsHash,
  executeMatchInstruction,
  executeMatchPlan,
  extractFinalOutcome,
  finalOutcomeMarketTerms,
  finalOutcomeProof,
  finalOutcomeSideStrategy,
  finalOutcomeStatKeys,
  finalOutcomeStrategy,
  finalOutcomeValidationPlan,
  findFinalOutcome,
  isFinalOutcomeRecord,
  marketIntentParamsFromScoreMarketTerms,
  marketTermsStrategy,
  refundBatchInstruction,
  refundBatchPlan,
  scoreMarketStatKeys,
  settleMatchedTradeInstruction,
  settleMatchedTradeParamsFromV2,
  settleMatchedTradePlan,
  settleTradeInstruction,
  settleTradeParamsFromV2,
  settleTradePlan,
  spreadMarketTerms,
  traderPredicate,
  validateStatV2Instruction,
  validationInputForMarket,
  ScoresStatValidationV2,
  type CreateIntentAccounts,
  type CreateTradeAccounts,
  type ExecuteMatchAccounts,
  type Scores,
  type SettleMatchedTradeAccounts,
  type SettleTradeAccounts,
  type StatValidationInput,
  type TxlineInstruction,
} from "../src/index.js";

describe("final outcome records", () => {
  it("detects only documented final outcome score records", () => {
    const score = finalScore(2, 1);

    expect(isFinalOutcomeRecord(score)).toBe(true);
    expect(isFinalOutcomeRecord({ ...score, action: "score_updated" })).toBe(false);
    expect(isFinalOutcomeRecord({ ...score, statusId: 99 })).toBe(false);
    expect(isFinalOutcomeRecord({ ...score, period: 1 })).toBe(false);
  });

  it("extracts participant1, participant2, and draw outcomes", () => {
    expect(extractFinalOutcome(finalScore(3, 1)).side).toBe("participant1");
    expect(extractFinalOutcome(finalScore(1, 2)).side).toBe("participant2");
    expect(extractFinalOutcome(finalScore(2, 2)).side).toBe("draw");
  });

  it("reports explicit errors for missing stats and non-final records", () => {
    expect(() => extractFinalOutcome({ ...finalScore(1, 0), stats: undefined })).toThrow(
      /no stats payload/u,
    );
    expect(() => extractFinalOutcome({ ...finalScore(1, 0), stats: { "1": 1 } })).toThrow(
      /missing stat key 2/u,
    );
    expect(() =>
      extractFinalOutcome({ ...finalScore(1, 0), action: "score_updated" }),
    ).toThrow(/action=game_finalised, statusId=100, period=100/u);
  });

  it("finds the final outcome in a score sequence", () => {
    const outcome = findFinalOutcome([
      { ...finalScore(0, 0), action: "score_updated", statusId: 1, period: 1 },
      finalScore(1, 2),
    ]);

    expect(outcome.side).toBe("participant2");
    expect(outcome.seq).toBe(941);
  });
});

describe("market terms and terms hashes", () => {
  it("maps score market terms deterministically to Devnet MarketIntentParams", () => {
    const terms = finalOutcomeMarketTerms(17_952_170, "participant2");
    const first = marketIntentParamsFromScoreMarketTerms(terms);
    const second = marketIntentParamsFromScoreMarketTerms(terms);

    expect(first).toEqual(second);
    expect(first).toMatchObject({
      fixtureId: 17_952_170,
      period: 100,
      statAKey: 2,
      statBKey: 1,
      predicate: traderPredicate(0, comparison.greaterThan()),
      op: binaryExpression.subtract(),
      negation: false,
    });
    expect(scoreMarketStatKeys(terms)).toEqual([2, 1]);

    const spread = spreadMarketTerms(
      17_952_170,
      "participant1",
      traderPredicate(-1, comparison.greaterThan()),
    );
    expect(marketIntentParamsFromScoreMarketTerms(spread).op).toEqual(
      binaryExpression.subtract(),
    );
  });

  it("validates caller-provided terms hashes without deriving a preimage", () => {
    const hash = hashBytes(40);

    expect([...ensureTermsHash(hash)]).toEqual([...hash]);
    expect(ensureTermsHash(hash)).not.toBe(hash);
    expect(() => ensureTermsHash(hash.slice(0, 31))).toThrow(/exactly 32 bytes/u);
    expect(() => ensureTermsHash([...hash.slice(0, 31), 300])).toThrow(
      /0..=255/u,
    );
  });
});

describe("strategy and proof assembly", () => {
  it("uses documented stat indexes for final-outcome strategies", () => {
    expect(binaryPredicate(finalOutcomeSideStrategy("participant1"))).toMatchObject({
      indexA: 0,
      indexB: 1,
      op: binaryExpression.subtract(),
      predicate: traderPredicate(0, comparison.greaterThan()),
    });
    expect(binaryPredicate(finalOutcomeSideStrategy("participant2"))).toMatchObject({
      indexA: 1,
      indexB: 0,
      op: binaryExpression.subtract(),
      predicate: traderPredicate(0, comparison.greaterThan()),
    });
    expect(binaryPredicate(finalOutcomeSideStrategy("draw"))).toMatchObject({
      indexA: 0,
      indexB: 1,
      op: binaryExpression.subtract(),
      predicate: traderPredicate(0, comparison.equalTo()),
    });
  });

  it("preserves V2 stat key order and builds final-outcome proof inputs", () => {
    const outcome = extractFinalOutcome(finalScore(2, 1));
    const validation = validationForOutcome(outcome);
    const proof = finalOutcomeProof(outcome, validation);

    expect(proof.statKeys).toEqual([1, 2]);
    expect(proof.payload.stats.map((leaf) => leaf.stat.key)).toEqual([1, 2]);
    expect(binaryPredicate(proof.strategy)).toMatchObject({
      indexA: 0,
      indexB: 1,
    });
  });

  it("rejects final-outcome proofs with wrong stat order", () => {
    const outcome = extractFinalOutcome(finalScore(2, 1));
    const validation = validationForOutcome(outcome, [2, 1]);

    expect(() => finalOutcomeProof(outcome, validation)).toThrow(/stat key order/u);
  });

  it("builds market strategies from the validation request order", () => {
    const terms = finalOutcomeMarketTerms(17_952_170, "participant2");
    const strategy = marketTermsStrategy(terms, [1, 2]);

    expect(binaryPredicate(strategy)).toMatchObject({
      indexA: 1,
      indexB: 0,
      predicate: traderPredicate(0, comparison.greaterThan()),
    });
  });

  it("preserves payload order while settlement inputs select stats by key", () => {
    const outcome = extractFinalOutcome(finalScore(1, 2));
    const terms = finalOutcomeMarketTerms(outcome.fixtureId, "participant2");
    const validation = validationForOutcome(outcome, [2, 1]);
    const payload = validationInputForMarket(validation, terms);
    const params = settleTradeParamsFromV2(44, payload, terms);

    expect(payload.stats.map((leaf) => leaf.stat.key)).toEqual([2, 1]);
    expect(params.statA.statToProve.key).toBe(2);
    expect(params.statB?.statToProve.key).toBe(1);
  });
});

describe("lifecycle plans", () => {
  it("wraps create intent and direct trade builders with terms-hash validation", () => {
    const terms = finalOutcomeMarketTerms(17_952_170, "participant1");
    const termsHash = hashBytes(80);

    expectPlanMatches(
      createIntentPlan(programId, createIntentAccounts(), {
        intentId: 9001,
        termsHash,
        depositAmount: 123_456,
        expirationTs: 1_781_129_999_999,
        claimPeriod: 42,
        terms,
      }),
      createIntentInstruction(programId, createIntentAccounts(), {
        intentId: 9001,
        termsHash,
        depositAmount: 123_456,
        expirationTs: 1_781_129_999_999,
        claimPeriod: 42,
        fixtureId: terms.fixtureId,
      }),
    );

    expectPlanMatches(
      createTradePlan(programId, createTradeAccounts(), {
        tradeId: 9002,
        stakeA: 111,
        stakeB: 222,
        tradeTermsHash: termsHash,
      }),
      createTradeInstruction(programId, createTradeAccounts(), {
        tradeId: 9002,
        stakeA: 111,
        stakeB: 222,
        tradeTermsHash: termsHash,
      }),
    );
  });

  it("wraps match, settlement, validation, refund, claim, and audit builders", () => {
    const outcome = extractFinalOutcome(finalScore(2, 1));
    const validation = validationForOutcome(outcome);
    const payload = validation.toValidationInput();
    const terms = finalOutcomeMarketTerms(outcome.fixtureId, "participant1");
    const root = key(72);

    expectPlanMatches(
      closeIntentPlan(programId, closeIntentAccounts()),
      closeIntentInstruction(programId, closeIntentAccounts()),
    );

    expectPlanMatches(
      executeMatchPlan(programId, executeMatchAccounts(), {
        tradeId: 9003,
        makerStake: 333,
        takerStake: 444,
      }),
      executeMatchInstruction(programId, executeMatchAccounts(), {
        tradeId: 9003,
        makerStake: 333,
        takerStake: 444,
      }),
    );

    expectPlanMatches(
      finalOutcomeValidationPlan(programId, root, validation, outcome),
      validateStatV2Instruction(
        programId,
        root,
        finalOutcomeProof(outcome, validation).payload,
        finalOutcomeStrategy(outcome),
      ),
    );

    expectPlanMatches(
      settleTradePlan(programId, settleTradeAccounts(), {
        tradeId: 9004,
        validationInput: payload,
        terms,
      }),
      settleTradeInstruction(
        programId,
        settleTradeAccounts(),
        settleTradeParamsFromV2(9004, payload, terms),
      ),
    );

    expectPlanMatches(
      settleMatchedTradePlan(programId, settleMatchedTradeAccounts(), {
        tradeId: 9005,
        validationInput: payload,
        terms,
      }),
      settleMatchedTradeInstruction(
        programId,
        settleMatchedTradeAccounts(),
        settleMatchedTradeParamsFromV2(9005, payload, terms),
      ),
    );

    expectPlanMatches(
      claimBatchLegacyPlan(programId, claimBatchLegacyAccounts(), {
        epochDay: 20_616,
        intervalIndex: 18,
        termsHash: hashBytes(90),
        winnerIsMaker: true,
        seq: outcome.seq,
        merkleProof: [],
      }),
      claimBatchLegacyInstruction(programId, claimBatchLegacyAccounts(), {
        epochDay: 20_616,
        intervalIndex: 18,
        termsHash: hashBytes(90),
        winnerIsMaker: true,
        seq: outcome.seq,
        merkleProof: [],
      }),
    );

    expectPlanMatches(
      claimViaResolutionPlan(programId, claimViaResolutionAccounts(), {
        epochDay: 20_616,
        intervalIndex: 18,
        merkleProof: [],
      }),
      claimViaResolutionInstruction(programId, claimViaResolutionAccounts(), {
        epochDay: 20_616,
        intervalIndex: 18,
        merkleProof: [],
      }),
    );

    expectPlanMatches(
      refundBatchPlan(programId, refundBatchAccounts()),
      refundBatchInstruction(programId, refundBatchAccounts()),
    );

    expectPlanMatches(
      auditTradeResultPlan(programId, auditTradeResultAccounts(), {
        validationInput: payload,
        terms,
      }),
      auditTradeResultInstruction(
        programId,
        auditTradeResultAccounts(),
        auditTradeResultParamsFromV2(payload, terms),
      ),
    );
  });
});

const programId = key(200);

function finalScore(participant1: number, participant2: number): Scores {
  return {
    fixtureId: 17_952_170,
    gameState: "ended",
    startTime: 1_781_100_000_000,
    isTeam: true,
    fixtureGroupId: 1,
    competitionId: 1,
    countryId: 1,
    sportId: 1,
    participant1IsHome: true,
    participant2Id: 20,
    participant1Id: 10,
    action: "game_finalised",
    id: 123,
    ts: 1_781_123_456_789,
    connectionId: 1,
    seq: 941,
    statusId: 100,
    period: 100,
    stats: {
      "1": participant1,
      "2": participant2,
    },
  };
}

function validationForOutcome(
  outcome: ReturnType<typeof extractFinalOutcome>,
  keys: readonly number[] = finalOutcomeStatKeys(outcome.config),
): ScoresStatValidationV2 {
  return ScoresStatValidationV2.fromResponse(keys, {
    ts: 1_781_123_456_789,
    statsToProve: keys.map((keyValue) => ({
      key: keyValue,
      value:
        keyValue === outcome.config.participant1GoalsStatKey
          ? outcome.participant1Score
          : outcome.participant2Score,
      period: outcome.config.period,
    })),
    eventStatRoot: hashBytes(20),
    summary: {
      fixtureId: outcome.fixtureId,
      updateStats: {
        updateCount: 1,
        minTimestamp: 1_781_123_456_789,
        maxTimestamp: 1_781_123_456_799,
      },
      eventStatsSubTreeRoot: hashBytes(10),
    },
    statProofs: keys.map(() => []),
    subTreeProof: [],
    mainTreeProof: [],
  });
}

function binaryPredicate(strategy: ReturnType<typeof finalOutcomeSideStrategy>) {
  const predicate = strategy.discretePredicates[0];
  if (!predicate || !("binary" in predicate)) {
    throw new Error("expected binary predicate");
  }
  return predicate.binary;
}

function expectPlanMatches(
  plan: { readonly instructions: readonly TxlineInstruction[] },
  expected: TxlineInstruction,
): void {
  const instruction = plan.instructions[0];
  expect(instruction).toBeDefined();
  expect(instruction!.accounts).toEqual(expected.accounts);
  expect([...instruction!.data]).toEqual([...expected.data]);
}

function createIntentAccounts(): CreateIntentAccounts {
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

function createTradeAccounts(): CreateTradeAccounts {
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

function executeMatchAccounts(): ExecuteMatchAccounts {
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

function settleTradeAccounts(): SettleTradeAccounts {
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

function settleMatchedTradeAccounts(): SettleMatchedTradeAccounts {
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

function claimBatchLegacyAccounts() {
  return {
    payer: key(121),
    dailyResolutionRoots: key(122),
    tokenMint: key(123),
    tokenProgram: key(124),
    systemProgram: key(125),
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

function key(base: number) {
  return addressFromBytes(new Uint8Array(32).fill(base));
}

function hashBytes(base: number): Uint8Array {
  return Uint8Array.from({ length: 32 }, (_value, index) => (base + index) & 0xff);
}
