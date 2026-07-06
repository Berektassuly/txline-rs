import type { TxlineClient } from "../client.js";
import { InvalidInputError, SolanaSafetyError } from "../errors.js";
import type {
  PurchaseQuoteRequest,
  PurchaseQuoteResponse,
} from "../http/models.js";
import { encodeWithDiscriminator } from "./codec.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  DevnetPdas,
  LEGACY_TOKEN_PROGRAM_ID,
  SYSTEM_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
} from "./pda.js";
import {
  readonly,
  readonlySigner,
  toAddress,
  writable,
  writableSigner,
  type AddressLike,
  type TxlineInstruction,
} from "./types.js";

export const MAX_QUOTE_TXLINE_AMOUNT = 100_000_000;

export const PURCHASE_SUBSCRIPTION_TOKEN_USDT_DISCRIMINATOR = [
  198, 251, 223, 9, 31, 184, 166, 188,
] as const;

export interface PurchaseSubscriptionTokenUsdtAccounts {
  readonly buyer: AddressLike;
  readonly backendAdmin: AddressLike;
  readonly usdtMint: AddressLike;
  readonly buyerUsdtAccount: AddressLike;
  readonly usdtTreasuryVault: AddressLike;
  readonly usdtTreasuryPda: AddressLike;
  readonly subscriptionTokenMint: AddressLike;
  readonly tokenTreasuryVault: AddressLike;
  readonly tokenTreasuryPda: AddressLike;
  readonly buyerTokenAccount: AddressLike;
  readonly tokenProgram: AddressLike;
  readonly token2022Program: AddressLike;
  readonly systemProgram: AddressLike;
  readonly associatedTokenProgram: AddressLike;
}

export function validateQuoteAmount(txlineAmount: number | bigint): void {
  if (typeof txlineAmount === "number" && !Number.isInteger(txlineAmount)) {
    throw new InvalidInputError(
      `txlineAmount must be an integer in 1..=${MAX_QUOTE_TXLINE_AMOUNT}`,
    );
  }
  const amount = BigInt(txlineAmount);
  if (amount <= 0n || amount > BigInt(MAX_QUOTE_TXLINE_AMOUNT)) {
    throw new InvalidInputError(
      `txlineAmount must be 1..=${MAX_QUOTE_TXLINE_AMOUNT}`,
    );
  }
}

export async function purchaseQuote(
  client: TxlineClient,
  buyerPubkey: AddressLike,
  txlineAmount: number | bigint,
): Promise<PurchaseQuoteResponse> {
  validateQuoteAmount(txlineAmount);
  const request: PurchaseQuoteRequest = {
    buyerPubkey: toAddress(buyerPubkey),
    txlineAmount: Number(txlineAmount),
  };
  return await client.postJson<PurchaseQuoteResponse>(
    "/guest/purchase/quote",
    request,
    false,
  );
}

export async function devnetPurchaseSubscriptionTokenUsdtAccounts(
  buyer: AddressLike,
  backendAdmin: AddressLike,
): Promise<PurchaseSubscriptionTokenUsdtAccounts> {
  const pdas = new DevnetPdas();
  return {
    buyer,
    backendAdmin,
    usdtMint: pdas.usdtMint,
    buyerUsdtAccount: (await pdas.userUsdtAta(buyer)).address,
    usdtTreasuryVault: (await pdas.usdtTreasuryVaultAta()).address,
    usdtTreasuryPda: (await pdas.usdtTreasury()).address,
    subscriptionTokenMint: pdas.txlMint,
    tokenTreasuryVault: (await pdas.tokenTreasuryVaultAta()).address,
    tokenTreasuryPda: (await pdas.tokenTreasuryV2()).address,
    buyerTokenAccount: (await pdas.userTxlAta(buyer)).address,
    tokenProgram: LEGACY_TOKEN_PROGRAM_ID,
    token2022Program: TOKEN_2022_PROGRAM_ID,
    systemProgram: SYSTEM_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  };
}

export function purchaseSubscriptionTokenUsdtInstruction(
  programId: AddressLike,
  accounts: PurchaseSubscriptionTokenUsdtAccounts,
  txlineAmount: number | bigint,
): TxlineInstruction {
  validateQuoteAmount(txlineAmount);
  return {
    programAddress: toAddress(programId),
    accounts: [
      writableSigner(accounts.buyer),
      readonlySigner(accounts.backendAdmin),
      readonly(accounts.usdtMint),
      writable(accounts.buyerUsdtAccount),
      writable(accounts.usdtTreasuryVault),
      readonly(accounts.usdtTreasuryPda),
      readonly(accounts.subscriptionTokenMint),
      writable(accounts.tokenTreasuryVault),
      readonly(accounts.tokenTreasuryPda),
      writable(accounts.buyerTokenAccount),
      readonly(accounts.tokenProgram),
      readonly(accounts.token2022Program),
      readonly(accounts.systemProgram),
      readonly(accounts.associatedTokenProgram),
    ],
    data: encodeWithDiscriminator(
      PURCHASE_SUBSCRIPTION_TOKEN_USDT_DISCRIMINATOR,
      (writer) => writer.putU64(txlineAmount),
    ),
  };
}

export function rawPurchaseQuoteTransactionBytesUnchecked(
  quote: PurchaseQuoteResponse,
): Uint8Array {
  const bytes = decodeBase64(quote.transactionBase64);
  if (bytes.length === 0) {
    throw new SolanaSafetyError(
      "purchase quote transaction decoded to an empty byte buffer",
    );
  }
  return bytes;
}

export function validatePurchaseQuoteFinancialShape(
  quote: PurchaseQuoteResponse,
): void {
  if (
    quote.baseUsdtCost < 0 ||
    quote.feeUsdtAmount < 0 ||
    quote.totalUsdtCharged < 0
  ) {
    throw new SolanaSafetyError("purchase quote contains negative USDT amounts");
  }
  const expected = quote.baseUsdtCost + quote.feeUsdtAmount;
  if (Math.abs(expected - quote.totalUsdtCharged) > 0.000_001) {
    throw new SolanaSafetyError(
      "purchase quote total does not equal base cost plus fee",
    );
  }
}

function decodeBase64(value: string): Uint8Array {
  try {
    const binary = globalThis.atob(value);
    return Uint8Array.from(binary, (char) => char.charCodeAt(0));
  } catch (cause) {
    throw new SolanaSafetyError("could not decode purchase quote transaction", {
      cause,
    });
  }
}
