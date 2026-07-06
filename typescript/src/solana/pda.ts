import { getProgramDerivedAddress } from "@solana/kit";
import {
  DEVNET_PROGRAM_ID,
  DEVNET_TXL_MINT,
  DEVNET_USDT_MINT,
} from "../config.js";
import {
  addressBytes,
  toAddress,
  type AddressLike,
} from "./types.js";
import type { Address } from "@solana/kit";

export const TOKEN_2022_PROGRAM_ID =
  "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
export const LEGACY_TOKEN_PROGRAM_ID =
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
export const ASSOCIATED_TOKEN_PROGRAM_ID =
  "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
export const SYSTEM_PROGRAM_ID = "11111111111111111111111111111111";
export const COMPUTE_BUDGET_PROGRAM_ID =
  "ComputeBudget111111111111111111111111111111";

export interface Pda {
  readonly address: Address;
  readonly bump: number;
}

export class DevnetPdas {
  readonly programId = toAddress(DEVNET_PROGRAM_ID);
  readonly txlMint = toAddress(DEVNET_TXL_MINT);
  readonly usdtMint = toAddress(DEVNET_USDT_MINT);

  async pricingMatrix(): Promise<Pda> {
    return await findPda(["pricing_matrix"], this.programId);
  }

  async tokenTreasuryV2(): Promise<Pda> {
    return await findPda(["token_treasury_v2"], this.programId);
  }

  async usdtTreasury(): Promise<Pda> {
    return await findPda(["usdt_treasury"], this.programId);
  }

  async tokenTreasuryVaultAta(): Promise<Pda> {
    return await token2022AssociatedTokenAddress(
      (await this.tokenTreasuryV2()).address,
      this.txlMint,
    );
  }

  async usdtTreasuryVaultAta(): Promise<Pda> {
    return await token2022AssociatedTokenAddress(
      (await this.usdtTreasury()).address,
      this.usdtMint,
    );
  }

  async userTxlAta(user: AddressLike): Promise<Pda> {
    return await token2022AssociatedTokenAddress(user, this.txlMint);
  }

  async userUsdtAta(user: AddressLike): Promise<Pda> {
    return await token2022AssociatedTokenAddress(user, this.usdtMint);
  }

  async dailyScoresRoots(epochDay: number): Promise<Pda> {
    return await findPda(
      ["daily_scores_roots", u16Le(epochDay)],
      this.programId,
    );
  }

  async dailyBatchRoots(epochDay: number): Promise<Pda> {
    return await findPda(
      ["daily_batch_roots", u16Le(epochDay)],
      this.programId,
    );
  }

  async dailyOddsMerkleRoots(epochDay: number): Promise<Pda> {
    return await this.dailyBatchRoots(epochDay);
  }

  async tenDailyFixturesRoots(epochDay: number): Promise<Pda> {
    const aligned = epochDay - (epochDay % 10);
    return await findPda(
      ["ten_daily_fixtures_roots", u16Le(aligned)],
      this.programId,
    );
  }
}

export async function findPda(
  seeds: readonly (string | Uint8Array)[],
  programAddress: AddressLike,
): Promise<Pda> {
  const [pda, bump] = await getProgramDerivedAddress({
    programAddress: toAddress(programAddress),
    seeds: seeds.map((seed) =>
      typeof seed === "string" ? new TextEncoder().encode(seed) : seed,
    ),
  });
  return { address: pda, bump };
}

export async function token2022AssociatedTokenAddress(
  owner: AddressLike,
  mint: AddressLike,
): Promise<Pda> {
  return await findPda(
    [
      addressBytes(owner),
      addressBytes(TOKEN_2022_PROGRAM_ID),
      addressBytes(mint),
    ],
    ASSOCIATED_TOKEN_PROGRAM_ID,
  );
}

export function u16Le(value: number): Uint8Array {
  if (!Number.isInteger(value) || value < 0 || value > 0xffff) {
    throw new RangeError("epoch day must fit in u16");
  }
  return Uint8Array.of(value & 0xff, (value >> 8) & 0xff);
}
