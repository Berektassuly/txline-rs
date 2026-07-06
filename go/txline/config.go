package txline

import "strings"

const (
	DEVNET_API_HOST       = "https://txline-dev.txodds.com"
	DEVNET_API_BASE       = "https://txline-dev.txodds.com/api"
	DEVNET_GUEST_AUTH_URL = "https://txline-dev.txodds.com/auth/guest/start"
	DEVNET_PROGRAM_ID     = "6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J"
	DEVNET_TXL_MINT       = "4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG"
	DEVNET_USDT_MINT      = "ELWTKspHKCnCfCiCiqYw1EDH77k8VCP74dK9qytG2Ujh"
	DEVNET_RPC_URL        = "https://api.devnet.solana.com"
)

type Network string

const Devnet Network = "devnet"

type Config struct {
	Network      Network
	APIHost      string
	APIBase      string
	GuestAuthURL string
	ProgramID    string
	TxLMint      string
	USDTMint     string
	RPCURL       string
}

func DevnetConfig() Config {
	return Config{
		Network:      Devnet,
		APIHost:      DEVNET_API_HOST,
		APIBase:      DEVNET_API_BASE,
		GuestAuthURL: DEVNET_GUEST_AUTH_URL,
		ProgramID:    DEVNET_PROGRAM_ID,
		TxLMint:      DEVNET_TXL_MINT,
		USDTMint:     DEVNET_USDT_MINT,
		RPCURL:       DEVNET_RPC_URL,
	}
}

func (c Config) WithRPCURL(rpcURL string) Config {
	c.RPCURL = rpcURL
	return c
}

func (c Config) validate() error {
	if c.Network != Devnet {
		return newError(ErrConfig, "only TxLINE Devnet is supported by this SDK build")
	}
	if c.APIHost != DEVNET_API_HOST ||
		c.APIBase != DEVNET_API_BASE ||
		c.GuestAuthURL != DEVNET_GUEST_AUTH_URL ||
		c.ProgramID != DEVNET_PROGRAM_ID ||
		c.TxLMint != DEVNET_TXL_MINT ||
		c.USDTMint != DEVNET_USDT_MINT {
		return newError(ErrConfig, "TxLINE Devnet config values must not be mixed with other networks")
	}
	if strings.TrimSpace(c.RPCURL) == "" {
		return newError(ErrConfig, "Solana RPC URL must not be empty")
	}
	if looksLikeMainnetRPCURL(c.RPCURL) {
		return newError(ErrConfig, "Solana RPC URL must be a Devnet RPC endpoint for this SDK build")
	}
	return nil
}

func looksLikeMainnetRPCURL(rpcURL string) bool {
	fields := strings.FieldsFunc(strings.ToLower(strings.TrimSpace(rpcURL)), func(r rune) bool {
		return !(r >= 'a' && r <= 'z') && !(r >= '0' && r <= '9')
	})
	for _, field := range fields {
		if field == "mainnet" || field == "mainnetbeta" {
			return true
		}
	}
	return false
}
