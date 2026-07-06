import type { ProofNode } from "../validation/proof.js";

export type ExtraFields = Record<string, unknown>;

export interface Fixture extends ExtraFields {
  Ts: number;
  StartTime: number;
  Competition: string;
  CompetitionId: number;
  FixtureGroupId: number;
  Participant1Id: number;
  Participant1: string;
  Participant2Id: number;
  Participant2: string;
  FixtureId: number;
  Participant1IsHome: boolean;
  GameState?: number;
}

export interface OddsPayload extends ExtraFields {
  FixtureId: number;
  MessageId: string;
  Ts: number;
  Bookmaker: string;
  BookmakerId: number;
  SuperOddsType: string;
  GameState?: string;
  InRunning: boolean;
  MarketParameters?: string;
  MarketPeriod?: string;
  PriceNames?: string[];
  Prices?: number[];
  Pct?: string[];
}

export interface PlayerStats {
  goals?: number;
  ownGoals?: number;
  penaltyAttempts?: number;
  penaltyGoals?: number;
  redCards?: number;
  shots?: number;
  yellowCards?: number;
}

export interface PlayerStatsForParticipants {
  Participant1?: Record<string, PlayerStats>;
  Participant2?: Record<string, PlayerStats>;
}

export interface Scores extends ExtraFields {
  fixtureId: number;
  gameState: string;
  startTime: number;
  isTeam: boolean;
  fixtureGroupId: number;
  competitionId: number;
  countryId: number;
  sportId: number;
  participant1IsHome: boolean;
  participant2Id: number;
  participant1Id: number;
  action: string;
  id: number;
  ts: number;
  connectionId: number;
  seq: number;
  statusId?: number;
  period?: number;
  coverageSecondaryData?: boolean;
  coverageType?: string;
  confirmed?: boolean;
  participant?: number;
  possession?: number;
  stats?: Record<string, number>;
  PlayerStats?: PlayerStatsForParticipants;
}

export interface UpdateStats {
  updateCount: number;
  minTimestamp: number;
  maxTimestamp: number;
}

export interface BatchMetadata {
  totalUpdateCount: number;
  numUniqueFixtures: number;
  overallBatchStartTs: number;
  overallBatchEndTs: number;
}

export interface FixtureBatchSummary {
  fixtureId: number;
  competitionId: number;
  competition: string;
  updateStats: UpdateStats;
  updateSubTreeRoot: string | readonly number[] | Uint8Array;
}

export interface FixtureValidation {
  snapshot: Fixture;
  summary: FixtureBatchSummary;
  subTreeProof?: ProofNode[];
  mainTreeProof?: ProofNode[];
}

export interface FixtureBatchValidation {
  metadata: BatchMetadata;
  proof?: ProofNode[];
}

export interface OddsBatchSummary {
  fixtureId: number;
  updateStats: UpdateStats;
  oddsSubTreeRoot: string | readonly number[] | Uint8Array;
}

export interface OddsValidation {
  odds: OddsPayload;
  summary: OddsBatchSummary;
  subTreeProof?: ProofNode[];
  mainTreeProof?: ProofNode[];
}

export interface PurchaseQuoteRequest {
  buyerPubkey: string;
  txlineAmount: number | bigint;
}

export interface PurchaseQuoteResponse {
  transactionBase64: string;
  baseUsdtCost: number;
  feeUsdtAmount: number;
  totalUsdtCharged: number;
}
