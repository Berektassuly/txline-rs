import { describe, expect, it } from "vitest";
import {
  rawPurchaseQuoteTransactionBytesUnchecked,
  validatePurchaseQuoteFinancialShape,
  verifyPurchaseTransactionBytes,
} from "../src/index.js";

describe("purchase quote safety", () => {
  it("does not expose raw quote bytes as checked bytes", () => {
    expect(() =>
      rawPurchaseQuoteTransactionBytesUnchecked({
        transactionBase64: "",
        baseUsdtCost: 1,
        feeUsdtAmount: 0,
        totalUsdtCharged: 1,
      }),
    ).toThrow(/empty byte buffer/u);
  });

  it("rejects malformed financial shape", () => {
    expect(() =>
      validatePurchaseQuoteFinancialShape({
        transactionBase64: "AQ==",
        baseUsdtCost: 1,
        feeUsdtAmount: 1,
        totalUsdtCharged: 1,
      }),
    ).toThrow(/total does not equal/u);
  });

  it("safe verification rejects malformed transaction bytes", async () => {
    await expect(
      verifyPurchaseTransactionBytes(new Uint8Array([1, 2, 3]), {
        txlineProgramId: "6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J",
        expectedBuyer: "11111111111111111111111111111111",
        expectedTxlineAmount: 1,
        expectedBackendSigner: "11111111111111111111111111111111",
      }),
    ).rejects.toThrow(/could not decode purchase transaction/u);
  });
});
