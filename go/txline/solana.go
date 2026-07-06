package txline

import (
	"encoding/base64"
	"fmt"

	"github.com/gagliardetto/solana-go"
)

const (
	Token2022ProgramIDString       = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
	LegacyTokenProgramIDString     = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
	AssociatedTokenProgramIDString = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
	SystemProgramIDString          = "11111111111111111111111111111111"
	ComputeBudgetProgramIDString   = "ComputeBudget111111111111111111111111111111"
)

type PublicKey = solana.PublicKey
type Instruction = solana.Instruction

func ParsePublicKey(value string) (solana.PublicKey, error) {
	key, err := solana.PublicKeyFromBase58(value)
	if err != nil {
		return solana.PublicKey{}, &Error{Kind: ErrSolana, Msg: fmt.Sprintf("invalid pubkey %s", value), Err: err}
	}
	return key, nil
}

func DevnetProgramPublicKey() solana.PublicKey {
	return solana.MustPublicKeyFromBase58(DEVNET_PROGRAM_ID)
}

func DevnetTxLMintPublicKey() solana.PublicKey {
	return solana.MustPublicKeyFromBase58(DEVNET_TXL_MINT)
}

func DevnetUSDTMintPublicKey() solana.PublicKey {
	return solana.MustPublicKeyFromBase58(DEVNET_USDT_MINT)
}

func meta(pubkey solana.PublicKey, writable, signer bool) *solana.AccountMeta {
	return solana.NewAccountMeta(pubkey, writable, signer)
}

func readonly(pubkey solana.PublicKey) *solana.AccountMeta {
	return meta(pubkey, false, false)
}

func writable(pubkey solana.PublicKey) *solana.AccountMeta {
	return meta(pubkey, true, false)
}

func writableSigner(pubkey solana.PublicKey) *solana.AccountMeta {
	return meta(pubkey, true, true)
}

func readonlySigner(pubkey solana.PublicKey) *solana.AccountMeta {
	return meta(pubkey, false, true)
}

func newInstruction(programID solana.PublicKey, accounts solana.AccountMetaSlice, data []byte) solana.Instruction {
	return solana.NewInstruction(programID, accounts, data)
}

func instructionData(ix solana.Instruction) ([]byte, error) {
	data, err := ix.Data()
	if err != nil {
		return nil, err
	}
	out := make([]byte, len(data))
	copy(out, data)
	return out, nil
}

func decodeQuoteTransactionBase64(value string) ([]byte, error) {
	bytes, err := base64.StdEncoding.DecodeString(value)
	if err != nil {
		return nil, &Error{Kind: ErrSolana, Msg: "could not decode purchase quote transaction", Err: err}
	}
	if len(bytes) == 0 {
		return nil, &Error{Kind: ErrSolana, Msg: "purchase quote transaction decoded to an empty byte buffer"}
	}
	return bytes, nil
}
