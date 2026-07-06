import { encodeWithDiscriminator } from "./codec.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
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

export interface CreateToken2022AssociatedTokenAccountAccounts {
  readonly payer: AddressLike;
  readonly associatedTokenAccount: AddressLike;
  readonly owner: AddressLike;
  readonly mint: AddressLike;
  readonly systemProgram?: AddressLike;
  readonly tokenProgram?: AddressLike;
}

export function createToken2022AssociatedTokenAccountInstruction(
  accounts: CreateToken2022AssociatedTokenAccountAccounts,
): TxlineInstruction {
  return {
    programAddress: toAddress(ASSOCIATED_TOKEN_PROGRAM_ID),
    accounts: [
      writableSigner(accounts.payer),
      writable(accounts.associatedTokenAccount),
      readonly(accounts.owner),
      readonly(accounts.mint),
      readonly(accounts.systemProgram ?? SYSTEM_PROGRAM_ID),
      readonly(accounts.tokenProgram ?? TOKEN_2022_PROGRAM_ID),
    ],
    data: encodeWithDiscriminator([]),
  };
}
