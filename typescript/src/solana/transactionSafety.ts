import {
  AccountRole,
  decompileTransactionMessage,
  getCompiledTransactionMessageDecoder,
  getPublicKeyFromAddress,
  getTransactionDecoder,
  verifySignature,
  type Address,
} from "@solana/kit";
import { DEVNET_PROGRAM_ID } from "../config.js";
import { SolanaSafetyError } from "../errors.js";
import type { PurchaseQuoteResponse } from "../http/models.js";
import {
  rawPurchaseQuoteTransactionBytesUnchecked,
  validatePurchaseQuoteFinancialShape,
  PURCHASE_SUBSCRIPTION_TOKEN_USDT_DISCRIMINATOR,
} from "./purchase.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  COMPUTE_BUDGET_PROGRAM_ID,
  DevnetPdas,
  LEGACY_TOKEN_PROGRAM_ID,
  SYSTEM_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
} from "./pda.js";
import {
  isSignerRole,
  toAddress,
  type AddressLike,
} from "./types.js";

export interface PurchaseTransactionSafetyConfig {
  readonly txlineProgramId: AddressLike;
  readonly expectedBuyer: AddressLike;
  readonly expectedTxlineAmount: number | bigint;
  readonly expectedBackendSigner: AddressLike;
}

export interface LowLevelPurchaseTransactionSafetyConfig {
  readonly txlineProgramId: AddressLike;
  readonly expectedBuyer: AddressLike;
  readonly expectedTxlineAmount: number | bigint;
  readonly expectedBackendSigner?: AddressLike;
}

export interface PurchaseTransactionSafetyReport {
  readonly feePayer: Address;
  readonly invokedPrograms: readonly Address[];
  readonly txlinePurchaseInstructionCount: number;
  readonly backendSignerPresent: boolean;
}

export interface ValidatedPurchaseQuote {
  readonly quote: PurchaseQuoteResponse;
  readonly safetyReport: PurchaseTransactionSafetyReport;
  readonly transactionBytes: Uint8Array;
}

export function devnetPurchaseSafetyConfig(options: {
  readonly expectedBuyer: AddressLike;
  readonly expectedTxlineAmount: number | bigint;
  readonly expectedBackendSigner: AddressLike;
}): PurchaseTransactionSafetyConfig {
  return {
    txlineProgramId: DEVNET_PROGRAM_ID,
    expectedBuyer: options.expectedBuyer,
    expectedTxlineAmount: options.expectedTxlineAmount,
    expectedBackendSigner: options.expectedBackendSigner,
  };
}

export async function validatedPurchaseQuote(
  quote: PurchaseQuoteResponse,
  config: PurchaseTransactionSafetyConfig,
): Promise<ValidatedPurchaseQuote> {
  validatePurchaseQuoteFinancialShape(quote);
  const transactionBytes = rawPurchaseQuoteTransactionBytesUnchecked(quote);
  const safetyReport = await verifyPurchaseTransactionBytes(transactionBytes, config);
  return { quote, safetyReport, transactionBytes };
}

export async function verifyPurchaseTransactionBase64(
  transactionBase64: string,
  config: PurchaseTransactionSafetyConfig,
): Promise<PurchaseTransactionSafetyReport> {
  return await verifyPurchaseTransactionBytes(
    rawPurchaseQuoteTransactionBytesUnchecked({
      transactionBase64,
      baseUsdtCost: 0,
      feeUsdtAmount: 0,
      totalUsdtCharged: 0,
    }),
    config,
  );
}

export async function verifyPurchaseTransactionBytes(
  transactionBytes: Uint8Array,
  config: PurchaseTransactionSafetyConfig,
): Promise<PurchaseTransactionSafetyReport> {
  if (!config.expectedBackendSigner) {
    throw new SolanaSafetyError(
      "safe purchase validation requires an expected backend signer",
    );
  }
  return await verifyPurchaseTransactionBytesLowLevelUncheckedBackendSigner(
    transactionBytes,
    config,
    true,
  );
}

export async function verifyPurchaseTransactionBytesLowLevelUncheckedBackendSigner(
  transactionBytes: Uint8Array,
  config: LowLevelPurchaseTransactionSafetyConfig,
  requireBackendSigner = false,
): Promise<PurchaseTransactionSafetyReport> {
  if (transactionBytes.length === 0) {
    throw new SolanaSafetyError(
      "purchase quote transaction decoded to an empty byte buffer",
    );
  }

  let transaction: ReturnType<ReturnType<typeof getTransactionDecoder>["decode"]>;
  try {
    transaction = getTransactionDecoder().decode(transactionBytes);
  } catch (cause) {
    throw new SolanaSafetyError(`could not decode purchase transaction: ${String(cause)}`, {
      cause,
    });
  }

  const compiled = getCompiledTransactionMessageDecoder().decode(
    transaction.messageBytes,
  );
  if (
    "addressTableLookups" in compiled &&
    Array.isArray(compiled.addressTableLookups) &&
    compiled.addressTableLookups.length > 0
  ) {
    throw new SolanaSafetyError(
      "purchase quote uses address table lookups; SDK cannot audit dynamically loaded accounts safely",
    );
  }

  const message = decompileTransactionMessage(compiled, {
    addressesByLookupTableAddress: {},
  });
  const feePayer = message.feePayer.address;
  const expectedBuyer = toAddress(config.expectedBuyer);
  if (feePayer !== expectedBuyer) {
    throw new SolanaSafetyError(
      "purchase transaction fee payer is not the expected buyer",
    );
  }

  const expectedBackend = config.expectedBackendSigner
    ? toAddress(config.expectedBackendSigner)
    : undefined;
  const backendSignerPresent = expectedBackend
    ? await backendSignaturePresent(transaction, expectedBackend)
    : false;
  if (requireBackendSigner && !backendSignerPresent) {
    throw new SolanaSafetyError(
      "purchase transaction is missing the expected backend signer signature",
    );
  }

  const txlineProgramId = toAddress(config.txlineProgramId);
  const allowedPrograms = new Set(
    allowedPurchasePrograms(txlineProgramId).map((program) => String(program)),
  );
  const invokedPrograms: Address[] = [];
  let purchaseInstructionCount = 0;

  for (const instruction of message.instructions) {
    const programAddress = instruction.programAddress;
    if (!allowedPrograms.has(String(programAddress))) {
      throw new SolanaSafetyError(
        `purchase transaction invokes unauthorized program ${programAddress}`,
      );
    }
    if (!invokedPrograms.some((program) => program === programAddress)) {
      invokedPrograms.push(programAddress);
    }

    rejectUnexpectedBuyerSigner(programAddress, txlineProgramId, instruction.accounts);

    if (programAddress === txlineProgramId) {
      purchaseInstructionCount += 1;
      verifyPurchaseInstructionData(
        Uint8Array.from(instruction.data ?? []),
        config,
      );
      await verifyPurchaseInstructionAccounts(
        instruction.accounts ?? [],
        config,
        expectedBackend,
      );
    }
  }

  if (purchaseInstructionCount !== 1) {
    throw new SolanaSafetyError(
      `purchase transaction must contain exactly one TxLINE purchase instruction, found ${purchaseInstructionCount}`,
    );
  }

  return {
    feePayer,
    invokedPrograms,
    txlinePurchaseInstructionCount: purchaseInstructionCount,
    backendSignerPresent,
  };
}

export function allowedPurchasePrograms(txlineProgramId: AddressLike): readonly Address[] {
  return [
    toAddress(txlineProgramId),
    toAddress(COMPUTE_BUDGET_PROGRAM_ID),
    toAddress(SYSTEM_PROGRAM_ID),
    toAddress(LEGACY_TOKEN_PROGRAM_ID),
    toAddress(TOKEN_2022_PROGRAM_ID),
    toAddress(ASSOCIATED_TOKEN_PROGRAM_ID),
  ];
}

async function backendSignaturePresent(
  transaction: ReturnType<ReturnType<typeof getTransactionDecoder>["decode"]>,
  expectedBackend: Address,
): Promise<boolean> {
  const signature = transaction.signatures[expectedBackend];
  if (!signature) {
    throw new SolanaSafetyError(
      "expected backend signer is not present in transaction accounts",
    );
  }
  if (signature.every((byte) => byte === 0)) {
    return false;
  }
  const publicKey = await getPublicKeyFromAddress(expectedBackend);
  const verified = await verifySignature(publicKey, signature, transaction.messageBytes);
  if (!verified) {
    throw new SolanaSafetyError("expected backend signer signature does not verify");
  }
  return true;
}

function rejectUnexpectedBuyerSigner(
  programAddress: Address,
  txlineProgramId: Address,
  accounts: readonly { readonly address: Address; readonly role: AccountRole }[] = [],
): void {
  const buyerAccount = accounts[0];
  const buyerIsSigner =
    buyerAccount !== undefined && isSignerRole(buyerAccount.role);
  if (
    buyerIsSigner &&
    programAddress !== txlineProgramId &&
    programAddress !== toAddress(ASSOCIATED_TOKEN_PROGRAM_ID)
  ) {
    throw new SolanaSafetyError(
      `buyer wallet is requested as signer for unauthorized program ${programAddress}`,
    );
  }
}

function verifyPurchaseInstructionData(
  data: Uint8Array,
  config: LowLevelPurchaseTransactionSafetyConfig,
): void {
  if (data.length !== 16) {
    throw new SolanaSafetyError(
      `purchase instruction data length is ${data.length}, expected 16`,
    );
  }
  for (let i = 0; i < PURCHASE_SUBSCRIPTION_TOKEN_USDT_DISCRIMINATOR.length; i += 1) {
    if (data[i] !== PURCHASE_SUBSCRIPTION_TOKEN_USDT_DISCRIMINATOR[i]) {
      throw new SolanaSafetyError(
        "TxLINE instruction is not purchase_subscription_token_usdt",
      );
    }
  }
  const amount = new DataView(data.buffer, data.byteOffset + 8, 8).getBigUint64(
    0,
    true,
  );
  if (amount !== BigInt(config.expectedTxlineAmount)) {
    throw new SolanaSafetyError(
      `purchase txlineAmount ${amount} does not match expected ${config.expectedTxlineAmount}`,
    );
  }
}

async function verifyPurchaseInstructionAccounts(
  accounts: readonly { readonly address: Address; readonly role: AccountRole }[],
  config: LowLevelPurchaseTransactionSafetyConfig,
  expectedBackend: Address | undefined,
): Promise<void> {
  if (accounts.length !== 14) {
    throw new SolanaSafetyError(
      `purchase instruction account count is ${accounts.length}, expected 14`,
    );
  }
  const pdas = new DevnetPdas();
  const expected = [
    toAddress(config.expectedBuyer),
    expectedBackend,
    pdas.usdtMint,
    (await pdas.userUsdtAta(config.expectedBuyer)).address,
    (await pdas.usdtTreasuryVaultAta()).address,
    (await pdas.usdtTreasury()).address,
    pdas.txlMint,
    (await pdas.tokenTreasuryVaultAta()).address,
    (await pdas.tokenTreasuryV2()).address,
    (await pdas.userTxlAta(config.expectedBuyer)).address,
    toAddress(LEGACY_TOKEN_PROGRAM_ID),
    toAddress(TOKEN_2022_PROGRAM_ID),
    toAddress(SYSTEM_PROGRAM_ID),
    toAddress(ASSOCIATED_TOKEN_PROGRAM_ID),
  ];

  expected.forEach((expectedAddress, index) => {
    const actual = accounts[index]?.address;
    if (expectedAddress && actual !== expectedAddress) {
      throw new SolanaSafetyError(
        `purchase instruction account ${index} is ${actual}, expected ${expectedAddress}`,
      );
    }
  });
}
