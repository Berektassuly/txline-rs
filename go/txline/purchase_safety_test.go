package txline

import (
	"encoding/base64"
	"errors"
	"testing"

	"github.com/gagliardetto/solana-go"
)

func TestPurchaseQuoteSafetyAcceptsSyntheticDevnetTransaction(t *testing.T) {
	buyer, backend := testPrivateKey(t), testPrivateKey(t)
	tx := signedPurchaseTransaction(t, buyer, backend, 1_000, nil)
	bytes, err := tx.MarshalBinary()
	if err != nil {
		t.Fatal(err)
	}
	config := DevnetPurchaseTransactionSafetyConfig(buyer.PublicKey(), 1_000, backend.PublicKey())

	report, err := VerifyPurchaseTransactionBytes(bytes, config)
	if err != nil {
		t.Fatalf("expected synthetic purchase transaction to pass: %v", err)
	}
	if report.FeePayer != buyer.PublicKey() || report.TxlinePurchaseInstructionCount != 1 || !report.BackendSignerPresent {
		t.Fatalf("unexpected report: %+v", report)
	}
}

func TestPurchaseQuoteSafetyRejectsBadShapes(t *testing.T) {
	buyer, backend := testPrivateKey(t), testPrivateKey(t)
	tx := signedPurchaseTransaction(t, buyer, backend, 1_000, nil)
	bytes, err := tx.MarshalBinary()
	if err != nil {
		t.Fatal(err)
	}
	config := DevnetPurchaseTransactionSafetyConfig(buyer.PublicKey(), 999, backend.PublicKey())
	if _, err := VerifyPurchaseTransactionBytes(bytes, config); !errors.Is(err, ErrSolana) {
		t.Fatalf("amount mismatch should be ErrSolana: %v", err)
	}

	config = DevnetPurchaseTransactionSafetyConfig(buyer.PublicKey(), 1_000, solana.NewWallet().PublicKey())
	if _, err := VerifyPurchaseTransactionBytes(bytes, config); !errors.Is(err, ErrSolana) {
		t.Fatalf("backend mismatch should be ErrSolana: %v", err)
	}

	rogue := newInstruction(solana.NewWallet().PublicKey(), nil, nil)
	tx = signedPurchaseTransaction(t, buyer, backend, 1_000, []solana.Instruction{rogue})
	bytes, err = tx.MarshalBinary()
	if err != nil {
		t.Fatal(err)
	}
	config = DevnetPurchaseTransactionSafetyConfig(buyer.PublicKey(), 1_000, backend.PublicKey())
	if _, err := VerifyPurchaseTransactionBytes(bytes, config); !errors.Is(err, ErrSolana) {
		t.Fatalf("rogue program should be ErrSolana: %v", err)
	}
}

func TestPurchaseQuoteResponseValidatedBytesAndFinancialShape(t *testing.T) {
	buyer, backend := testPrivateKey(t), testPrivateKey(t)
	tx := signedPurchaseTransaction(t, buyer, backend, 1_000, nil)
	bytes, err := tx.MarshalBinary()
	if err != nil {
		t.Fatal(err)
	}
	quote := PurchaseQuoteResponse{
		TransactionBase64: base64.StdEncoding.EncodeToString(bytes),
		BaseUSDTCost:      1.0,
		FeeUSDTAmount:     0.25,
		TotalUSDTCharged:  1.25,
	}
	config := DevnetPurchaseTransactionSafetyConfig(buyer.PublicKey(), 1_000, backend.PublicKey())
	checked, err := quote.ValidatedTransactionBytes(config)
	if err != nil {
		t.Fatal(err)
	}
	if string(checked) != string(bytes) {
		t.Fatal("validated bytes changed")
	}
	quote.TotalUSDTCharged = 2.0
	if err := quote.ValidateFinancialShape(); !errors.Is(err, ErrSolana) {
		t.Fatalf("bad financial shape should fail: %v", err)
	}
}

func testPrivateKey(t *testing.T) solana.PrivateKey {
	t.Helper()
	key, err := solana.NewRandomPrivateKey()
	if err != nil {
		t.Fatal(err)
	}
	return key
}

func signedPurchaseTransaction(t *testing.T, buyer, backend solana.PrivateKey, amount uint64, extra []solana.Instruction) *solana.Transaction {
	t.Helper()
	accounts, err := DevnetPurchaseSubscriptionTokenUSDTAccounts(buyer.PublicKey(), backend.PublicKey())
	if err != nil {
		t.Fatal(err)
	}
	purchase, err := PurchaseSubscriptionTokenUSDTInstruction(DevnetProgramPublicKey(), accounts, amount)
	if err != nil {
		t.Fatal(err)
	}
	instructions := []solana.Instruction{purchase}
	instructions = append(instructions, extra...)
	var hash solana.Hash
	hash[0] = 7
	tx, err := solana.NewTransaction(instructions, hash, solana.TransactionPayer(buyer.PublicKey()))
	if err != nil {
		t.Fatal(err)
	}
	_, err = tx.Sign(func(key solana.PublicKey) *solana.PrivateKey {
		switch key {
		case buyer.PublicKey():
			return &buyer
		case backend.PublicKey():
			return &backend
		default:
			return nil
		}
	})
	if err != nil {
		t.Fatal(err)
	}
	return tx
}
