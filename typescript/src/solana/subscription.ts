import { InvalidInputError } from "../errors.js";
import { encodeWithDiscriminator } from "./codec.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  DevnetPdas,
  SYSTEM_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
} from "./pda.js";
import {
  readonly,
  toAddress,
  writable,
  writableSigner,
  type AddressLike,
  type TxlineInstruction,
} from "./types.js";

export const SUBSCRIBE_DISCRIMINATOR = [
  254, 28, 191, 138, 156, 179, 183, 53,
] as const;

export interface SubscribeAccounts {
  readonly user: AddressLike;
  readonly pricingMatrix: AddressLike;
  readonly tokenMint: AddressLike;
  readonly userTokenAccount: AddressLike;
  readonly tokenTreasuryVault: AddressLike;
  readonly tokenTreasuryPda: AddressLike;
  readonly tokenProgram: AddressLike;
  readonly systemProgram: AddressLike;
  readonly associatedTokenProgram: AddressLike;
}

export interface SubscribeParams {
  readonly serviceLevelId: number;
  readonly weeks: number;
}

export function validateSubscriptionWeeks(weeks: number): void {
  if (!Number.isInteger(weeks) || weeks < 4 || weeks % 4 !== 0 || weeks > 255) {
    throw new InvalidInputError(
      "subscription duration must be at least 4 weeks and a multiple of 4",
    );
  }
}

export async function devnetSubscribeAccounts(
  user: AddressLike,
): Promise<SubscribeAccounts> {
  const pdas = new DevnetPdas();
  return {
    user,
    pricingMatrix: (await pdas.pricingMatrix()).address,
    tokenMint: pdas.txlMint,
    userTokenAccount: (await pdas.userTxlAta(user)).address,
    tokenTreasuryVault: (await pdas.tokenTreasuryVaultAta()).address,
    tokenTreasuryPda: (await pdas.tokenTreasuryV2()).address,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
    systemProgram: SYSTEM_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  };
}

export function subscribeInstruction(
  programId: AddressLike,
  accounts: SubscribeAccounts,
  params: SubscribeParams,
): TxlineInstruction {
  validateSubscriptionWeeks(params.weeks);
  return {
    programAddress: toAddress(programId),
    accounts: [
      writableSigner(accounts.user),
      readonly(accounts.pricingMatrix),
      readonly(accounts.tokenMint),
      writable(accounts.userTokenAccount),
      writable(accounts.tokenTreasuryVault),
      readonly(accounts.tokenTreasuryPda),
      readonly(accounts.tokenProgram),
      readonly(accounts.systemProgram),
      readonly(accounts.associatedTokenProgram),
    ],
    data: encodeWithDiscriminator(SUBSCRIBE_DISCRIMINATOR, (writer) => {
      writer.putU16(params.serviceLevelId);
      writer.putU8(params.weeks);
    }),
  };
}
