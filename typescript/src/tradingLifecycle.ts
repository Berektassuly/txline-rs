import { InvalidInputError, ValidationPayloadError } from "./errors.js";
import type { Scores } from "./http/models.js";
import {
  auditTradeResultInstruction,
  claimBatchLegacyInstruction,
  claimViaResolutionInstruction,
  closeIntentInstruction,
  createIntentInstruction,
  createTradeInstruction,
  executeMatchInstruction,
  refundBatchInstruction,
  settleMatchedTradeInstruction,
  settleTradeInstruction,
  type AuditTradeResultAccounts,
  type AuditTradeResultParams,
  type ClaimBatchLegacyAccounts,
  type ClaimBatchLegacyParams,
  type ClaimViaResolutionAccounts,
  type ClaimViaResolutionParams,
  type CloseIntentAccounts,
  type CreateIntentAccounts,
  type CreateIntentParams,
  type CreateTradeAccounts,
  type CreateTradeParams,
  type ExecuteMatchAccounts,
  type ExecuteMatchParams,
  type MarketIntentParams,
  type RefundBatchAccounts,
  type SettleMatchedTradeAccounts,
  type SettleMatchedTradeParams,
  type SettleTradeAccounts,
  type SettleTradeParams,
} from "./solana/trading.js";
import type { AddressLike, TxlineInstruction } from "./solana/types.js";
import {
  devnetValidateStatV2Instruction,
  validateStatV2Instruction,
} from "./solana/validation.js";
import type { FixtureSummaryInput, StatTermInput } from "./validation/legacy.js";
import type { ProofNode } from "./validation/proof.js";
import {
  binaryExpression,
  comparison,
  strategyBuilder,
  traderPredicate,
  type BinaryExpression,
  type NDimensionalStrategy,
  type TraderPredicate,
} from "./validation/strategy.js";
import {
  ScoresStatValidationV2,
  type StatValidationInput,
} from "./validation/v2.js";

export const FINAL_OUTCOME_ACTION = "game_finalised";
export const FINAL_OUTCOME_STATUS_ID = 100;
export const FINAL_OUTCOME_PERIOD = 100;

export type MarketSide = "participant1" | "participant2" | "draw";

export type ScoreMarketKind = "final_outcome" | "total_goals" | "spread";

export interface FinalOutcomeConfig {
  readonly participant1GoalsStatKey: number;
  readonly participant2GoalsStatKey: number;
  readonly period: number;
}

export interface FinalOutcome {
  readonly fixtureId: number;
  readonly seq: number;
  readonly participant1Score: number;
  readonly participant2Score: number;
  readonly side: MarketSide;
  readonly config: FinalOutcomeConfig;
}

export interface ScoreMarketTerms {
  readonly fixtureId: number;
  readonly kind: ScoreMarketKind;
  readonly period: number;
  readonly statAKey: number;
  readonly statBKey?: number;
  readonly predicate: TraderPredicate;
  readonly op?: BinaryExpression;
  readonly negation: boolean;
}

export type TermsHashLike = Uint8Array | readonly number[];

export interface LifecyclePlan {
  readonly name: string;
  readonly instructions: readonly TxlineInstruction[];
  readonly nextSteps: readonly string[];
  readonly callerBoundaries: readonly string[];
}

export interface SettlementProofInputs {
  readonly ts: number;
  readonly fixtureSummary: FixtureSummaryInput;
  readonly fixtureProof: readonly ProofNode[];
  readonly mainTreeProof: readonly ProofNode[];
  readonly statA: StatTermInput;
  readonly statB?: StatTermInput;
}

export interface FinalOutcomeProof {
  readonly outcome: FinalOutcome;
  readonly statKeys: readonly number[];
  readonly payload: StatValidationInput;
  readonly strategy: NDimensionalStrategy;
}

export interface CreateIntentPlanOptions {
  readonly intentId: number | bigint;
  readonly termsHash: TermsHashLike;
  readonly depositAmount: number | bigint;
  readonly expirationTs: number | bigint;
  readonly claimPeriod: number;
  readonly terms: ScoreMarketTerms;
}

export interface CreateTradePlanOptions {
  readonly tradeId: number | bigint;
  readonly stakeA: number | bigint;
  readonly stakeB: number | bigint;
  readonly tradeTermsHash: TermsHashLike;
}

export interface ClaimBatchLegacyPlanOptions {
  readonly epochDay: number;
  readonly intervalIndex: number;
  readonly termsHash: TermsHashLike;
  readonly winnerIsMaker: boolean;
  readonly seq: number;
  readonly merkleProof: readonly ProofNode[];
}

export interface SettlementPlanOptions {
  readonly tradeId: number | bigint;
  readonly validationInput: StatValidationInput;
  readonly terms: ScoreMarketTerms;
}

export interface AuditTradeResultPlanOptions {
  readonly validationInput: StatValidationInput;
  readonly terms: ScoreMarketTerms;
}

export function defaultSoccerFinalOutcomeConfig(): FinalOutcomeConfig {
  return {
    participant1GoalsStatKey: 1,
    participant2GoalsStatKey: 2,
    period: FINAL_OUTCOME_PERIOD,
  };
}

export function isFinalOutcomeRecord(score: Scores): boolean {
  return (
    score.action === FINAL_OUTCOME_ACTION &&
    score.statusId === FINAL_OUTCOME_STATUS_ID &&
    score.period === FINAL_OUTCOME_PERIOD
  );
}

export function extractFinalOutcome(
  score: Scores,
  config: FinalOutcomeConfig = defaultSoccerFinalOutcomeConfig(),
): FinalOutcome {
  if (!isFinalOutcomeRecord(score)) {
    throw new InvalidInputError(
      "score record is not final-outcome settlement data; expected action=game_finalised, statusId=100, period=100",
    );
  }
  if (!Number.isInteger(score.seq) || score.seq <= 0) {
    throw new InvalidInputError("final outcome seq must be positive");
  }
  const participant1Score = scoreStatValue(
    score,
    config.participant1GoalsStatKey,
  );
  const participant2Score = scoreStatValue(
    score,
    config.participant2GoalsStatKey,
  );
  const side: MarketSide =
    participant1Score > participant2Score
      ? "participant1"
      : participant2Score > participant1Score
        ? "participant2"
        : "draw";

  return {
    fixtureId: score.fixtureId,
    seq: score.seq,
    participant1Score,
    participant2Score,
    side,
    config,
  };
}

export function findFinalOutcome(
  scores: readonly Scores[],
  config: FinalOutcomeConfig = defaultSoccerFinalOutcomeConfig(),
): FinalOutcome {
  const score = scores.find(isFinalOutcomeRecord);
  if (!score) {
    throw new InvalidInputError("no final outcome score record found");
  }
  return extractFinalOutcome(score, config);
}

export function finalOutcomeStatKeys(
  config: FinalOutcomeConfig = defaultSoccerFinalOutcomeConfig(),
): readonly number[] {
  return [
    config.participant1GoalsStatKey,
    config.participant2GoalsStatKey,
  ];
}

export function scoreMarketStatKeys(terms: ScoreMarketTerms): readonly number[] {
  validateScoreMarketTerms(terms);
  return terms.statBKey === undefined
    ? [terms.statAKey]
    : [terms.statAKey, terms.statBKey];
}

export function finalOutcomeMarketTerms(
  fixtureId: number,
  side: MarketSide,
  config: FinalOutcomeConfig = defaultSoccerFinalOutcomeConfig(),
): ScoreMarketTerms {
  validatePositiveInteger(fixtureId, "fixtureId");
  const greaterThanZero = traderPredicate(0, comparison.greaterThan());
  if (side === "participant1") {
    return {
      fixtureId,
      kind: "final_outcome",
      period: config.period,
      statAKey: config.participant1GoalsStatKey,
      statBKey: config.participant2GoalsStatKey,
      predicate: greaterThanZero,
      op: binaryExpression.subtract(),
      negation: false,
    };
  }
  if (side === "participant2") {
    return {
      fixtureId,
      kind: "final_outcome",
      period: config.period,
      statAKey: config.participant2GoalsStatKey,
      statBKey: config.participant1GoalsStatKey,
      predicate: greaterThanZero,
      op: binaryExpression.subtract(),
      negation: false,
    };
  }
  return {
    fixtureId,
    kind: "final_outcome",
    period: config.period,
    statAKey: config.participant1GoalsStatKey,
    statBKey: config.participant2GoalsStatKey,
    predicate: traderPredicate(0, comparison.equalTo()),
    op: binaryExpression.subtract(),
    negation: false,
  };
}

export function totalGoalsMarketTerms(
  fixtureId: number,
  predicate: TraderPredicate,
  config: FinalOutcomeConfig = defaultSoccerFinalOutcomeConfig(),
): ScoreMarketTerms {
  validatePositiveInteger(fixtureId, "fixtureId");
  return {
    fixtureId,
    kind: "total_goals",
    period: config.period,
    statAKey: config.participant1GoalsStatKey,
    statBKey: config.participant2GoalsStatKey,
    predicate,
    op: binaryExpression.add(),
    negation: false,
  };
}

export function spreadMarketTerms(
  fixtureId: number,
  side: MarketSide,
  predicate: TraderPredicate,
  config: FinalOutcomeConfig = defaultSoccerFinalOutcomeConfig(),
): ScoreMarketTerms {
  validatePositiveInteger(fixtureId, "fixtureId");
  if (side === "draw") {
    throw new InvalidInputError("spread markets require participant1 or participant2");
  }
  const participant1Side = side === "participant1";
  return {
    fixtureId,
    kind: "spread",
    period: config.period,
    statAKey: participant1Side
      ? config.participant1GoalsStatKey
      : config.participant2GoalsStatKey,
    statBKey: participant1Side
      ? config.participant2GoalsStatKey
      : config.participant1GoalsStatKey,
    predicate,
    op: binaryExpression.subtract(),
    negation: false,
  };
}

export function marketIntentParamsFromScoreMarketTerms(
  terms: ScoreMarketTerms,
): MarketIntentParams {
  validateScoreMarketTerms(terms);
  return {
    fixtureId: terms.fixtureId,
    period: terms.period,
    statAKey: terms.statAKey,
    ...(terms.statBKey !== undefined ? { statBKey: terms.statBKey } : {}),
    predicate: terms.predicate,
    ...(terms.op !== undefined ? { op: terms.op } : {}),
    negation: terms.negation,
  };
}

export function finalOutcomeSideStrategy(side: MarketSide): NDimensionalStrategy {
  if (side === "participant1") {
    return strategyBuilder(2)
      .binary(
        0,
        1,
        binaryExpression.subtract(),
        traderPredicate(0, comparison.greaterThan()),
      )
      .build();
  }
  if (side === "participant2") {
    return strategyBuilder(2)
      .binary(
        1,
        0,
        binaryExpression.subtract(),
        traderPredicate(0, comparison.greaterThan()),
      )
      .build();
  }
  return strategyBuilder(2)
    .binary(
      0,
      1,
      binaryExpression.subtract(),
      traderPredicate(0, comparison.equalTo()),
    )
    .build();
}

export function finalOutcomeStrategy(
  outcome: FinalOutcome,
): NDimensionalStrategy {
  return finalOutcomeSideStrategy(outcome.side);
}

export function marketTermsStrategy(
  terms: ScoreMarketTerms,
  requestedStatKeys: readonly number[] = scoreMarketStatKeys(terms),
): NDimensionalStrategy {
  validateScoreMarketTerms(terms);
  if (terms.negation) {
    throw new ValidationPayloadError(
      "N-dimensional validation strategies do not encode MarketIntentParams.negation; express the predicate directly before building a validation instruction",
    );
  }
  const indexA = indexOfStatKey(requestedStatKeys, terms.statAKey);
  const builder = strategyBuilder(requestedStatKeys.length);
  if (terms.statBKey === undefined) {
    return builder.single(indexA, terms.predicate).build();
  }
  if (terms.op === undefined) {
    throw new ValidationPayloadError("binary market terms require an operator");
  }
  const indexB = indexOfStatKey(requestedStatKeys, terms.statBKey);
  return builder.binary(indexA, indexB, terms.op, terms.predicate).build();
}

export function validationInputForMarket(
  validation: ScoresStatValidationV2,
  terms: ScoreMarketTerms,
): StatValidationInput {
  validateScoreMarketTerms(terms);
  const requested = validation.requestedStatKeys();
  const missing = scoreMarketStatKeys(terms).filter(
    (statKey) => !requested.includes(statKey),
  );
  if (missing.length > 0) {
    throw new ValidationPayloadError(
      `V2 validation payload is missing market stat keys ${missing.join(",")}`,
    );
  }
  const payload = validation.toValidationInput();
  if (payload.fixtureSummary.fixtureId !== terms.fixtureId) {
    throw new ValidationPayloadError(
      `validation fixtureId ${payload.fixtureSummary.fixtureId} does not match market fixtureId ${terms.fixtureId}`,
    );
  }
  return payload;
}

export function settlementInputsFromV2(
  payload: StatValidationInput,
  terms: ScoreMarketTerms,
): SettlementProofInputs {
  validateScoreMarketTerms(terms);
  if (payload.fixtureSummary.fixtureId !== terms.fixtureId) {
    throw new ValidationPayloadError(
      `validation fixtureId ${payload.fixtureSummary.fixtureId} does not match market fixtureId ${terms.fixtureId}`,
    );
  }
  const statA = statTermFromPayload(payload, terms.statAKey, terms.period);
  const statB =
    terms.statBKey === undefined
      ? undefined
      : statTermFromPayload(payload, terms.statBKey, terms.period);
  return {
    ts: payload.ts,
    fixtureSummary: payload.fixtureSummary,
    fixtureProof: payload.fixtureProof,
    mainTreeProof: payload.mainTreeProof,
    statA,
    ...(statB !== undefined ? { statB } : {}),
  };
}

export function finalOutcomeProof(
  outcome: FinalOutcome,
  validation: ScoresStatValidationV2,
): FinalOutcomeProof {
  const statKeys = finalOutcomeStatKeys(outcome.config);
  const requested = validation.requestedStatKeys();
  if (!sameNumberSequence(requested, statKeys)) {
    throw new ValidationPayloadError(
      `final outcome proof stat key order ${requested.join(",")} does not match expected order ${statKeys.join(",")}`,
    );
  }
  const payload = validation.toValidationInput();
  validateFinalOutcomePayload(outcome, payload);
  return {
    outcome,
    statKeys,
    payload,
    strategy: finalOutcomeStrategy(outcome),
  };
}

export function finalOutcomeValidationPlan(
  programId: AddressLike,
  dailyScoresMerkleRoots: AddressLike,
  validation: ScoresStatValidationV2,
  outcome: FinalOutcome,
): LifecyclePlan {
  const proof = finalOutcomeProof(outcome, validation);
  const instruction = validateStatV2Instruction(
    programId,
    dailyScoresMerkleRoots,
    proof.payload,
    proof.strategy,
  );
  return singleInstructionPlan("validate_stat_v2_final_outcome", instruction, [
    "simulate validate_stat_v2 against Devnet before using the result for settlement",
    "use caller-owned market accounts for settlement, claim, refund, or audit instructions",
  ]);
}

export async function devnetFinalOutcomeValidationPlan(
  programId: AddressLike,
  validation: ScoresStatValidationV2,
  outcome: FinalOutcome,
): Promise<LifecyclePlan> {
  const proof = finalOutcomeProof(outcome, validation);
  const instruction = await devnetValidateStatV2Instruction(
    programId,
    proof.payload,
    proof.strategy,
  );
  return singleInstructionPlan("validate_stat_v2_final_outcome", instruction, [
    "simulate validate_stat_v2 against Devnet before using the result for settlement",
    "use caller-owned market accounts for settlement, claim, refund, or audit instructions",
  ]);
}

export function ensureTermsHash(value: TermsHashLike): Uint8Array {
  const bytes = bytesFromTermsHash(value);
  if (bytes.length !== 32) {
    throw new InvalidInputError(
      "terms_hash must be exactly 32 bytes; pass the caller-provided market hash",
    );
  }
  return bytes;
}

export function createIntentPlan(
  programId: AddressLike,
  accounts: CreateIntentAccounts,
  options: CreateIntentPlanOptions,
): LifecyclePlan {
  validateScoreMarketTerms(options.terms);
  const params: CreateIntentParams = {
    intentId: options.intentId,
    termsHash: ensureTermsHash(options.termsHash),
    depositAmount: options.depositAmount,
    expirationTs: options.expirationTs,
    claimPeriod: options.claimPeriod,
    fixtureId: options.terms.fixtureId,
  };
  return singleInstructionPlan(
    "create_intent",
    createIntentInstruction(programId, accounts, params),
    [
      "the maker signs and submits the instruction with caller-supplied intent accounts",
      "the coordinating application stores the terms hash preimage and market metadata",
    ],
  );
}

export function closeIntentPlan(
  programId: AddressLike,
  accounts: CloseIntentAccounts,
): LifecyclePlan {
  return singleInstructionPlan(
    "close_intent",
    closeIntentInstruction(programId, accounts),
    ["the authority signs and submits the close instruction for the explicit intent account"],
  );
}

export function createTradePlan(
  programId: AddressLike,
  accounts: CreateTradeAccounts,
  options: CreateTradePlanOptions,
): LifecyclePlan {
  const params: CreateTradeParams = {
    tradeId: options.tradeId,
    stakeA: options.stakeA,
    stakeB: options.stakeB,
    tradeTermsHash: ensureTermsHash(options.tradeTermsHash),
  };
  return singleInstructionPlan(
    "create_trade",
    createTradeInstruction(programId, accounts, params),
    [
      "both traders and the authority sign according to the Devnet instruction account metas",
      "the coordinating application stores the trade terms hash preimage",
    ],
  );
}

export function executeMatchPlan(
  programId: AddressLike,
  accounts: ExecuteMatchAccounts,
  params: ExecuteMatchParams,
): LifecyclePlan {
  return singleInstructionPlan(
    "execute_match",
    executeMatchInstruction(programId, accounts, params),
    ["the solver submits the match using caller-supplied maker and taker intent accounts"],
  );
}

export function settleTradePlan(
  programId: AddressLike,
  accounts: SettleTradeAccounts,
  options: SettlementPlanOptions,
): LifecyclePlan {
  const params = settleTradeParamsFromV2(
    options.tradeId,
    options.validationInput,
    options.terms,
  );
  return singleInstructionPlan(
    "settle_trade",
    settleTradeInstruction(programId, accounts, params),
    [
      "the winner signs and submits the direct-trade settlement instruction",
      "the proof payload must come from the TxLINE stat-validation endpoint for the observed score seq",
    ],
  );
}

export function settleMatchedTradePlan(
  programId: AddressLike,
  accounts: SettleMatchedTradeAccounts,
  options: SettlementPlanOptions,
): LifecyclePlan {
  const params = settleMatchedTradeParamsFromV2(
    options.tradeId,
    options.validationInput,
    options.terms,
  );
  return singleInstructionPlan(
    "settle_matched_trade",
    settleMatchedTradeInstruction(programId, accounts, params),
    [
      "the winner signs and submits the matched-trade settlement instruction",
      "the application supplies matched trade, vault, token, and winner token accounts",
    ],
  );
}

export function claimViaResolutionPlan(
  programId: AddressLike,
  accounts: ClaimViaResolutionAccounts,
  params: ClaimViaResolutionParams,
): LifecyclePlan {
  return singleInstructionPlan(
    "claim_via_resolution",
    claimViaResolutionInstruction(programId, accounts, params),
    ["the caller supplies the resolution-root account and Merkle proof"],
  );
}

export function claimBatchLegacyPlan(
  programId: AddressLike,
  accounts: ClaimBatchLegacyAccounts,
  options: ClaimBatchLegacyPlanOptions,
): LifecyclePlan {
  const params: ClaimBatchLegacyParams = {
    epochDay: options.epochDay,
    intervalIndex: options.intervalIndex,
    termsHash: ensureTermsHash(options.termsHash),
    winnerIsMaker: options.winnerIsMaker,
    seq: options.seq,
    merkleProof: options.merkleProof,
  };
  return singleInstructionPlan(
    "claim_batch_legacy",
    claimBatchLegacyInstruction(programId, accounts, params),
    ["the caller supplies the legacy resolution proof and token accounts"],
  );
}

export function refundBatchPlan(
  programId: AddressLike,
  accounts: RefundBatchAccounts,
): LifecyclePlan {
  return singleInstructionPlan(
    "refund_batch",
    refundBatchInstruction(programId, accounts),
    ["the payer signs the refund instruction for the caller-supplied token accounts"],
  );
}

export function auditTradeResultPlan(
  programId: AddressLike,
  accounts: AuditTradeResultAccounts,
  options: AuditTradeResultPlanOptions,
): LifecyclePlan {
  const params = auditTradeResultParamsFromV2(
    options.validationInput,
    options.terms,
  );
  return singleInstructionPlan(
    "audit_trade_result",
    auditTradeResultInstruction(programId, accounts, params),
    ["the payer signs a read or audit instruction against the caller-supplied scores root"],
  );
}

export function settleTradeParamsFromV2(
  tradeId: number | bigint,
  validationInput: StatValidationInput,
  terms: ScoreMarketTerms,
): SettleTradeParams {
  if (terms.negation) {
    throw new ValidationPayloadError(
      "direct trade settlement params do not carry market negation",
    );
  }
  const settlement = settlementInputsFromV2(validationInput, terms);
  return {
    tradeId,
    ts: settlement.ts,
    fixtureSummary: settlement.fixtureSummary,
    fixtureProof: settlement.fixtureProof,
    mainTreeProof: settlement.mainTreeProof,
    predicate: terms.predicate,
    statA: settlement.statA,
    ...(settlement.statB !== undefined ? { statB: settlement.statB } : {}),
    ...(terms.op !== undefined ? { op: terms.op } : {}),
  };
}

export function settleMatchedTradeParamsFromV2(
  tradeId: number | bigint,
  validationInput: StatValidationInput,
  terms: ScoreMarketTerms,
): SettleMatchedTradeParams {
  const settlement = settlementInputsFromV2(validationInput, terms);
  return {
    tradeId,
    ts: settlement.ts,
    fixtureSummary: settlement.fixtureSummary,
    fixtureProof: settlement.fixtureProof,
    mainTreeProof: settlement.mainTreeProof,
    statA: settlement.statA,
    ...(settlement.statB !== undefined ? { statB: settlement.statB } : {}),
    terms: marketIntentParamsFromScoreMarketTerms(terms),
  };
}

export function auditTradeResultParamsFromV2(
  validationInput: StatValidationInput,
  terms: ScoreMarketTerms,
): AuditTradeResultParams {
  const settlement = settlementInputsFromV2(validationInput, terms);
  return {
    terms: marketIntentParamsFromScoreMarketTerms(terms),
    fixtureSummary: settlement.fixtureSummary,
    mainTreeProof: settlement.mainTreeProof,
    fixtureProof: settlement.fixtureProof,
    statA: settlement.statA,
    ...(settlement.statB !== undefined ? { statB: settlement.statB } : {}),
    ts: settlement.ts,
  };
}

function singleInstructionPlan(
  name: string,
  instruction: TxlineInstruction,
  nextSteps: readonly string[],
): LifecyclePlan {
  return {
    name,
    instructions: [instruction],
    nextSteps: [...nextSteps],
    callerBoundaries: [
      "trading PDAs, escrow accounts, vaults, signers, and terms-hash preimages are supplied by the application",
      "this SDK builds Devnet instructions only; it does not sign, simulate, or submit transactions",
    ],
  };
}

function validateScoreMarketTerms(terms: ScoreMarketTerms): void {
  validatePositiveInteger(terms.fixtureId, "fixtureId");
  validateU16(terms.period, "period");
  validateU32(terms.statAKey, "statAKey");
  if (terms.statBKey !== undefined) {
    validateU32(terms.statBKey, "statBKey");
  }
  if ((terms.statBKey === undefined) !== (terms.op === undefined)) {
    throw new InvalidInputError(
      "statBKey and op must either both be set or both be absent",
    );
  }
}

function validateFinalOutcomePayload(
  outcome: FinalOutcome,
  payload: StatValidationInput,
): void {
  if (payload.fixtureSummary.fixtureId !== outcome.fixtureId) {
    throw new ValidationPayloadError(
      `final outcome proof fixtureId ${payload.fixtureSummary.fixtureId} does not match outcome fixtureId ${outcome.fixtureId}`,
    );
  }
  if (payload.stats.length !== 2) {
    throw new ValidationPayloadError(
      `final outcome proof must contain exactly 2 stats, got ${payload.stats.length}`,
    );
  }
  const expected = [
    {
      key: outcome.config.participant1GoalsStatKey,
      value: outcome.participant1Score,
      period: outcome.config.period,
    },
    {
      key: outcome.config.participant2GoalsStatKey,
      value: outcome.participant2Score,
      period: outcome.config.period,
    },
  ];
  expected.forEach((expectedStat, index) => {
    const actual = payload.stats[index]?.stat;
    if (!actual) {
      throw new ValidationPayloadError(
        `final outcome proof is missing stat ${index}`,
      );
    }
    if (actual.key !== expectedStat.key) {
      throw new ValidationPayloadError(
        `final outcome proof stat ${index} key ${actual.key} does not match expected key ${expectedStat.key}`,
      );
    }
    if (actual.value !== expectedStat.value) {
      throw new ValidationPayloadError(
        `final outcome proof stat ${index} value ${actual.value} does not match observed value ${expectedStat.value}`,
      );
    }
    if (actual.period !== expectedStat.period) {
      throw new ValidationPayloadError(
        `final outcome proof stat ${index} period ${actual.period} does not match expected period ${expectedStat.period}`,
      );
    }
  });
}

function statTermFromPayload(
  payload: StatValidationInput,
  statKey: number,
  expectedPeriod: number,
): StatTermInput {
  const matches = payload.stats.filter((leaf) => leaf.stat.key === statKey);
  if (matches.length === 0) {
    throw new ValidationPayloadError(
      `validation payload does not contain stat key ${statKey}`,
    );
  }
  if (matches.length > 1) {
    throw new ValidationPayloadError(
      `validation payload contains duplicate stat key ${statKey}`,
    );
  }
  const leaf = matches[0]!;
  if (leaf.stat.period !== expectedPeriod) {
    throw new ValidationPayloadError(
      `stat key ${statKey} period ${leaf.stat.period} does not match market period ${expectedPeriod}`,
    );
  }
  return {
    statToProve: leaf.stat,
    eventStatRoot: payload.eventStatRoot,
    statProof: leaf.statProof,
  };
}

function scoreStatValue(score: Scores, statKey: number): number {
  const stats = score.stats;
  if (!stats) {
    throw new InvalidInputError("final outcome score record has no stats payload");
  }
  const value = stats[String(statKey)];
  if (value === undefined) {
    throw new InvalidInputError(
      `final outcome score record is missing stat key ${statKey}`,
    );
  }
  if (!Number.isFinite(value)) {
    throw new InvalidInputError(
      `final outcome score stat key ${statKey} is not a finite number`,
    );
  }
  return value;
}

function bytesFromTermsHash(value: TermsHashLike): Uint8Array {
  if (value instanceof Uint8Array) {
    return new Uint8Array(value);
  }
  const bytes = Array.from(value, (byte, index) => {
    if (!Number.isInteger(byte) || byte < 0 || byte > 255) {
      throw new InvalidInputError(
        `terms_hash byte ${index} must be an integer in 0..=255`,
      );
    }
    return byte;
  });
  return Uint8Array.from(bytes);
}

function indexOfStatKey(
  requestedStatKeys: readonly number[],
  statKey: number,
): number {
  const index = requestedStatKeys.indexOf(statKey);
  if (index === -1) {
    throw new ValidationPayloadError(
      `requested stat keys do not include market stat key ${statKey}`,
    );
  }
  return index;
}

function sameNumberSequence(
  left: readonly number[],
  right: readonly number[],
): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function validatePositiveInteger(value: number, name: string): void {
  if (!Number.isInteger(value) || value <= 0) {
    throw new InvalidInputError(`${name} must be a positive integer`);
  }
}

function validateU16(value: number, name: string): void {
  if (!Number.isInteger(value) || value < 0 || value > 0xffff) {
    throw new InvalidInputError(`${name} must fit into the Devnet IDL u16 field`);
  }
}

function validateU32(value: number, name: string): void {
  if (!Number.isInteger(value) || value < 0 || value > 0xffffffff) {
    throw new InvalidInputError(`${name} must fit into the Devnet IDL u32 field`);
  }
}
