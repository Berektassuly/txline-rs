import { encodeWithDiscriminator } from "./codec.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  DevnetPdas,
  LEGACY_TOKEN_PROGRAM_ID,
  SYSTEM_PROGRAM_ID,
} from "./pda.js";
import {
  readonly,
  toAddress,
  writable,
  writableSigner,
  type AddressLike,
  type TxlineInstruction,
} from "./types.js";

export const REQUEST_DEVNET_FAUCET_DISCRIMINATOR = [
  49, 178, 104, 8, 23, 120, 186, 21,
] as const;

export interface RequestDevnetFaucetAccounts {
  readonly user: AddressLike;
  readonly faucetTracker: AddressLike;
  readonly usdtMint: AddressLike;
  readonly userUsdtAta: AddressLike;
  readonly usdtTreasuryPda: AddressLike;
  readonly tokenProgram: AddressLike;
  readonly associatedTokenProgram: AddressLike;
  readonly systemProgram: AddressLike;
}

export async function devnetRequestFaucetAccounts(
  user: AddressLike,
  faucetTracker: AddressLike,
): Promise<RequestDevnetFaucetAccounts> {
  const pdas = new DevnetPdas();
  return {
    user,
    faucetTracker,
    usdtMint: pdas.usdtMint,
    userUsdtAta: (await pdas.userUsdtAta(user)).address,
    usdtTreasuryPda: (await pdas.usdtTreasury()).address,
    tokenProgram: LEGACY_TOKEN_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    systemProgram: SYSTEM_PROGRAM_ID,
  };
}

export function requestDevnetFaucetInstruction(
  programId: AddressLike,
  accounts: RequestDevnetFaucetAccounts,
): TxlineInstruction {
  return {
    programAddress: toAddress(programId),
    accounts: [
      writableSigner(accounts.user),
      writable(accounts.faucetTracker),
      writable(accounts.usdtMint),
      writable(accounts.userUsdtAta),
      readonly(accounts.usdtTreasuryPda),
      readonly(accounts.tokenProgram),
      readonly(accounts.associatedTokenProgram),
      readonly(accounts.systemProgram),
    ],
    data: encodeWithDiscriminator(REQUEST_DEVNET_FAUCET_DISCRIMINATOR),
  };
}
