import { decodeHash32, type Hash32Like, type ProofNode } from "../validation/proof.js";
import type {
  FixtureSummaryInput,
  StatTermInput,
} from "../validation/legacy.js";
import type {
  BinaryExpression,
  TraderPredicate,
} from "../validation/strategy.js";
import {
  encodeBinaryExpression,
  encodeProofVec,
  encodeScoresBatchSummary,
  encodeStatTerm,
  encodeTraderPredicate,
  encodeWithDiscriminator,
} from "./codec.js";
import {
  readonly,
  toAddress,
  writable,
  writableSigner,
  type AddressLike,
  type TxlineInstruction,
} from "./types.js";

export const CREATE_INTENT_DISCRIMINATOR = [
  216, 214, 79, 121, 23, 194, 96, 104,
] as const;
export const CREATE_TRADE_DISCRIMINATOR = [
  183, 82, 24, 245, 248, 30, 204, 246,
] as const;
export const EXECUTE_MATCH_DISCRIMINATOR = [
  76, 47, 91, 223, 20, 10, 147, 232,
] as const;
export const CLOSE_INTENT_DISCRIMINATOR = [
  112, 245, 154, 249, 57, 126, 54, 122,
] as const;
export const SETTLE_TRADE_DISCRIMINATOR = [
  252, 176, 98, 248, 73, 123, 8, 157,
] as const;
export const SETTLE_MATCHED_TRADE_DISCRIMINATOR = [
  191, 233, 149, 116, 32, 239, 18, 65,
] as const;
export const CLAIM_VIA_RESOLUTION_DISCRIMINATOR = [
  98, 206, 250, 87, 151, 135, 162, 181,
] as const;
export const CLAIM_BATCH_LEGACY_DISCRIMINATOR = [
  254, 101, 89, 255, 169, 75, 207, 66,
] as const;
export const REFUND_BATCH_DISCRIMINATOR = [
  227, 54, 194, 2, 78, 8, 104, 29,
] as const;
export const AUDIT_TRADE_RESULT_DISCRIMINATOR = [
  50, 242, 243, 5, 209, 75, 76, 91,
] as const;

export interface CreateIntentAccounts {
  readonly maker: AddressLike;
  readonly orderIntent: AddressLike;
  readonly intentVault: AddressLike;
  readonly makerTokenAccount: AddressLike;
  readonly tokenMint: AddressLike;
  readonly tokenTreasuryPda: AddressLike;
  readonly tokenProgram: AddressLike;
  readonly systemProgram: AddressLike;
}

export interface CreateIntentParams {
  readonly intentId: number | bigint;
  readonly termsHash: Hash32Like;
  readonly depositAmount: number | bigint;
  readonly expirationTs: number | bigint;
  readonly claimPeriod: number;
  readonly fixtureId: number | bigint;
}

export function createIntentInstruction(
  programId: AddressLike,
  accounts: CreateIntentAccounts,
  params: CreateIntentParams,
): TxlineInstruction {
  return {
    programAddress: toAddress(programId),
    accounts: [
      writableSigner(accounts.maker),
      writable(accounts.orderIntent),
      writable(accounts.intentVault),
      writable(accounts.makerTokenAccount),
      readonly(accounts.tokenMint),
      readonly(accounts.tokenTreasuryPda),
      readonly(accounts.tokenProgram),
      readonly(accounts.systemProgram),
    ],
    data: encodeWithDiscriminator(CREATE_INTENT_DISCRIMINATOR, (writer) => {
      writer.putU64(params.intentId);
      writer.writeBytes(decodeHash32(params.termsHash));
      writer.putU64(params.depositAmount);
      writer.putI64(params.expirationTs);
      writer.putU16(params.claimPeriod);
      writer.putI64(params.fixtureId);
    }),
  };
}

export interface CreateTradeAccounts {
  readonly authority: AddressLike;
  readonly traderA: AddressLike;
  readonly traderB: AddressLike;
  readonly traderATokenAccount: AddressLike;
  readonly traderBTokenAccount: AddressLike;
  readonly tradeEscrow: AddressLike;
  readonly escrowVault: AddressLike;
  readonly stakeTokenMint: AddressLike;
  readonly tokenTreasuryPda: AddressLike;
  readonly tokenProgram: AddressLike;
  readonly systemProgram: AddressLike;
}

export interface CreateTradeParams {
  readonly tradeId: number | bigint;
  readonly stakeA: number | bigint;
  readonly stakeB: number | bigint;
  readonly tradeTermsHash: Hash32Like;
}

export function createTradeInstruction(
  programId: AddressLike,
  accounts: CreateTradeAccounts,
  params: CreateTradeParams,
): TxlineInstruction {
  return {
    programAddress: toAddress(programId),
    accounts: [
      writableSigner(accounts.authority),
      writableSigner(accounts.traderA),
      writableSigner(accounts.traderB),
      writable(accounts.traderATokenAccount),
      writable(accounts.traderBTokenAccount),
      writable(accounts.tradeEscrow),
      writable(accounts.escrowVault),
      readonly(accounts.stakeTokenMint),
      readonly(accounts.tokenTreasuryPda),
      readonly(accounts.tokenProgram),
      readonly(accounts.systemProgram),
    ],
    data: encodeWithDiscriminator(CREATE_TRADE_DISCRIMINATOR, (writer) => {
      writer.putU64(params.tradeId);
      writer.putU64(params.stakeA);
      writer.putU64(params.stakeB);
      writer.writeBytes(decodeHash32(params.tradeTermsHash));
    }),
  };
}

export interface ExecuteMatchAccounts {
  readonly solver: AddressLike;
  readonly makerIntent: AddressLike;
  readonly takerIntent: AddressLike;
  readonly makerVault: AddressLike;
  readonly takerVault: AddressLike;
  readonly matchedTrade: AddressLike;
  readonly tradeVault: AddressLike;
  readonly tokenMint: AddressLike;
  readonly tokenProgram: AddressLike;
  readonly systemProgram: AddressLike;
}

export interface ExecuteMatchParams {
  readonly tradeId: number | bigint;
  readonly makerStake: number | bigint;
  readonly takerStake: number | bigint;
}

export function executeMatchInstruction(
  programId: AddressLike,
  accounts: ExecuteMatchAccounts,
  params: ExecuteMatchParams,
): TxlineInstruction {
  return {
    programAddress: toAddress(programId),
    accounts: [
      writableSigner(accounts.solver),
      writable(accounts.makerIntent),
      writable(accounts.takerIntent),
      writable(accounts.makerVault),
      writable(accounts.takerVault),
      writable(accounts.matchedTrade),
      writable(accounts.tradeVault),
      readonly(accounts.tokenMint),
      readonly(accounts.tokenProgram),
      readonly(accounts.systemProgram),
    ],
    data: encodeWithDiscriminator(EXECUTE_MATCH_DISCRIMINATOR, (writer) => {
      writer.putU64(params.tradeId);
      writer.putU64(params.makerStake);
      writer.putU64(params.takerStake);
    }),
  };
}

export interface CloseIntentAccounts {
  readonly maker: AddressLike;
  readonly authority: AddressLike;
  readonly orderIntent: AddressLike;
  readonly intentVault: AddressLike;
  readonly makerTokenAccount: AddressLike;
  readonly tokenMint: AddressLike;
  readonly tokenProgram: AddressLike;
  readonly tokenTreasuryPda: AddressLike;
}

export function closeIntentInstruction(
  programId: AddressLike,
  accounts: CloseIntentAccounts,
): TxlineInstruction {
  return {
    programAddress: toAddress(programId),
    accounts: [
      writable(accounts.maker),
      writableSigner(accounts.authority),
      writable(accounts.orderIntent),
      writable(accounts.intentVault),
      writable(accounts.makerTokenAccount),
      readonly(accounts.tokenMint),
      readonly(accounts.tokenProgram),
      readonly(accounts.tokenTreasuryPda),
    ],
    data: encodeWithDiscriminator(CLOSE_INTENT_DISCRIMINATOR),
  };
}

export interface SettleTradeAccounts {
  readonly winner: AddressLike;
  readonly dailyScoresMerkleRoots: AddressLike;
  readonly tradeEscrow: AddressLike;
  readonly escrowVault: AddressLike;
  readonly winnerTokenAccount: AddressLike;
  readonly tokenMint: AddressLike;
  readonly tokenTreasuryPda: AddressLike;
  readonly tokenProgram: AddressLike;
  readonly systemProgram: AddressLike;
}

export interface SettleTradeParams {
  readonly tradeId: number | bigint;
  readonly ts: number | bigint;
  readonly fixtureSummary: FixtureSummaryInput;
  readonly fixtureProof: readonly ProofNode[];
  readonly mainTreeProof: readonly ProofNode[];
  readonly predicate: TraderPredicate;
  readonly statA: StatTermInput;
  readonly statB?: StatTermInput;
  readonly op?: BinaryExpression;
}

export function settleTradeInstruction(
  programId: AddressLike,
  accounts: SettleTradeAccounts,
  params: SettleTradeParams,
): TxlineInstruction {
  return {
    programAddress: toAddress(programId),
    accounts: [
      writableSigner(accounts.winner),
      readonly(accounts.dailyScoresMerkleRoots),
      writable(accounts.tradeEscrow),
      writable(accounts.escrowVault),
      writable(accounts.winnerTokenAccount),
      readonly(accounts.tokenMint),
      readonly(accounts.tokenTreasuryPda),
      readonly(accounts.tokenProgram),
      readonly(accounts.systemProgram),
    ],
    data: encodeWithDiscriminator(SETTLE_TRADE_DISCRIMINATOR, (writer) => {
      writer.putU64(params.tradeId);
      writer.putI64(params.ts);
      encodeScoresBatchSummary(writer, params.fixtureSummary);
      encodeProofVec(writer, params.fixtureProof);
      encodeProofVec(writer, params.mainTreeProof);
      encodeTraderPredicate(writer, params.predicate);
      encodeStatTerm(writer, params.statA);
      writer.putOption(params.statB, encodeStatTerm);
      writer.putOption(params.op, encodeBinaryExpression);
    }),
  };
}

export interface SettleMatchedTradeAccounts {
  readonly winner: AddressLike;
  readonly dailyScoresMerkleRoots: AddressLike;
  readonly matchedTrade: AddressLike;
  readonly tradeVault: AddressLike;
  readonly winnerTokenAccount: AddressLike;
  readonly tokenMint: AddressLike;
  readonly tokenTreasuryPda: AddressLike;
  readonly tokenProgram: AddressLike;
  readonly systemProgram: AddressLike;
}

export interface SettleMatchedTradeParams {
  readonly tradeId: number | bigint;
  readonly ts: number | bigint;
  readonly fixtureSummary: FixtureSummaryInput;
  readonly fixtureProof: readonly ProofNode[];
  readonly mainTreeProof: readonly ProofNode[];
  readonly statA: StatTermInput;
  readonly statB?: StatTermInput;
  readonly terms: MarketIntentParams;
}

export function settleMatchedTradeInstruction(
  programId: AddressLike,
  accounts: SettleMatchedTradeAccounts,
  params: SettleMatchedTradeParams,
): TxlineInstruction {
  return {
    programAddress: toAddress(programId),
    accounts: [
      writableSigner(accounts.winner),
      readonly(accounts.dailyScoresMerkleRoots),
      writable(accounts.matchedTrade),
      writable(accounts.tradeVault),
      writable(accounts.winnerTokenAccount),
      readonly(accounts.tokenMint),
      readonly(accounts.tokenTreasuryPda),
      readonly(accounts.tokenProgram),
      readonly(accounts.systemProgram),
    ],
    data: encodeWithDiscriminator(SETTLE_MATCHED_TRADE_DISCRIMINATOR, (writer) => {
      writer.putU64(params.tradeId);
      writer.putI64(params.ts);
      encodeScoresBatchSummary(writer, params.fixtureSummary);
      encodeProofVec(writer, params.fixtureProof);
      encodeProofVec(writer, params.mainTreeProof);
      encodeStatTerm(writer, params.statA);
      writer.putOption(params.statB, encodeStatTerm);
      encodeMarketIntentParams(writer, params.terms);
    }),
  };
}

export interface ClaimViaResolutionAccounts {
  readonly winner: AddressLike;
  readonly dailyResolutionRoots: AddressLike;
  readonly matchedTrade: AddressLike;
  readonly tradeVault: AddressLike;
  readonly winnerTokenAccount: AddressLike;
  readonly tokenProgram: AddressLike;
}

export interface ClaimViaResolutionParams {
  readonly epochDay: number;
  readonly intervalIndex: number;
  readonly merkleProof: readonly ProofNode[];
}

export function claimViaResolutionInstruction(
  programId: AddressLike,
  accounts: ClaimViaResolutionAccounts,
  params: ClaimViaResolutionParams,
): TxlineInstruction {
  return {
    programAddress: toAddress(programId),
    accounts: [
      writableSigner(accounts.winner),
      readonly(accounts.dailyResolutionRoots),
      writable(accounts.matchedTrade),
      writable(accounts.tradeVault),
      writable(accounts.winnerTokenAccount),
      readonly(accounts.tokenProgram),
    ],
    data: encodeWithDiscriminator(CLAIM_VIA_RESOLUTION_DISCRIMINATOR, (writer) => {
      writer.putU16(params.epochDay);
      writer.putU16(params.intervalIndex);
      encodeProofVec(writer, params.merkleProof);
    }),
  };
}

export interface ClaimBatchLegacyAccounts {
  readonly payer: AddressLike;
  readonly dailyResolutionRoots: AddressLike;
  readonly tokenMint: AddressLike;
  readonly tokenProgram: AddressLike;
  readonly systemProgram: AddressLike;
}

export interface ClaimBatchLegacyParams {
  readonly epochDay: number;
  readonly intervalIndex: number;
  readonly termsHash: Hash32Like;
  readonly winnerIsMaker: boolean;
  readonly seq: number;
  readonly merkleProof: readonly ProofNode[];
}

export function claimBatchLegacyInstruction(
  programId: AddressLike,
  accounts: ClaimBatchLegacyAccounts,
  params: ClaimBatchLegacyParams,
): TxlineInstruction {
  return {
    programAddress: toAddress(programId),
    accounts: [
      writableSigner(accounts.payer),
      readonly(accounts.dailyResolutionRoots),
      readonly(accounts.tokenMint),
      readonly(accounts.tokenProgram),
      readonly(accounts.systemProgram),
    ],
    data: encodeWithDiscriminator(CLAIM_BATCH_LEGACY_DISCRIMINATOR, (writer) => {
      writer.putU16(params.epochDay);
      writer.putU16(params.intervalIndex);
      writer.writeBytes(decodeHash32(params.termsHash));
      writer.putBool(params.winnerIsMaker);
      writer.putU32(params.seq);
      encodeProofVec(writer, params.merkleProof);
    }),
  };
}

export interface RefundBatchAccounts {
  readonly payer: AddressLike;
  readonly tokenMint: AddressLike;
  readonly tokenProgram: AddressLike;
  readonly systemProgram: AddressLike;
}

export function refundBatchInstruction(
  programId: AddressLike,
  accounts: RefundBatchAccounts,
): TxlineInstruction {
  return {
    programAddress: toAddress(programId),
    accounts: [
      writableSigner(accounts.payer),
      readonly(accounts.tokenMint),
      readonly(accounts.tokenProgram),
      readonly(accounts.systemProgram),
    ],
    data: encodeWithDiscriminator(REFUND_BATCH_DISCRIMINATOR),
  };
}

export interface AuditTradeResultAccounts {
  readonly payer: AddressLike;
  readonly dailyScoresMerkleRoots: AddressLike;
}

export interface AuditTradeResultParams {
  readonly terms: MarketIntentParams;
  readonly fixtureSummary: FixtureSummaryInput;
  readonly mainTreeProof: readonly ProofNode[];
  readonly fixtureProof: readonly ProofNode[];
  readonly statA: StatTermInput;
  readonly statB?: StatTermInput;
  readonly ts: number | bigint;
}

export function auditTradeResultInstruction(
  programId: AddressLike,
  accounts: AuditTradeResultAccounts,
  params: AuditTradeResultParams,
): TxlineInstruction {
  return {
    programAddress: toAddress(programId),
    accounts: [writableSigner(accounts.payer), readonly(accounts.dailyScoresMerkleRoots)],
    data: encodeWithDiscriminator(AUDIT_TRADE_RESULT_DISCRIMINATOR, (writer) => {
      encodeMarketIntentParams(writer, params.terms);
      encodeScoresBatchSummary(writer, params.fixtureSummary);
      encodeProofVec(writer, params.mainTreeProof);
      encodeProofVec(writer, params.fixtureProof);
      encodeStatTerm(writer, params.statA);
      writer.putOption(params.statB, encodeStatTerm);
      writer.putI64(params.ts);
    }),
  };
}

export interface MarketIntentParams {
  readonly fixtureId: number | bigint;
  readonly period: number;
  readonly statAKey: number;
  readonly statBKey?: number;
  readonly predicate: TraderPredicate;
  readonly op?: BinaryExpression;
  readonly negation: boolean;
}

function encodeMarketIntentParams(
  writer: Parameters<typeof encodeScoresBatchSummary>[0],
  terms: MarketIntentParams,
): void {
  writer.putI64(terms.fixtureId);
  writer.putU16(terms.period);
  writer.putU32(terms.statAKey);
  writer.putOption(terms.statBKey, (out, value) => out.putU32(value));
  encodeTraderPredicate(writer, terms.predicate);
  writer.putOption(terms.op, encodeBinaryExpression);
  writer.putBool(terms.negation);
}
