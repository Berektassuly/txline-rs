import { ConfigError } from "./errors.js";

export const DEVNET_API_HOST = "https://txline-dev.txodds.com";
export const DEVNET_API_BASE = "https://txline-dev.txodds.com/api";
export const DEVNET_GUEST_AUTH_URL =
  "https://txline-dev.txodds.com/auth/guest/start";
export const DEVNET_PROGRAM_ID = "6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J";
export const DEVNET_TXL_MINT = "4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG";
export const DEVNET_USDT_MINT = "ELWTKspHKCnCfCiCiqYw1EDH77k8VCP74dK9qytG2Ujh";
export const DEVNET_RPC_URL = "https://api.devnet.solana.com";

export const MAINNET_REFERENCE = Object.freeze({
  apiHost: "https://txline.txodds.com",
  apiBase: "https://txline.txodds.com/api",
  guestAuthUrl: "https://txline.txodds.com/auth/guest/start",
  programId: "9ExbZjAapQww1vfcisDmrngPinHTEfpjYRWMunJgcKaA",
  txlMint: "Zhw9TVKp68a1QrftncMSd6ELXKDtpVMNuMGr1jNwdeL",
  usdtMint: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
});

export type Network = "devnet";

export interface TxlineConfig {
  readonly network: Network;
  readonly apiHost: string;
  readonly apiBase: string;
  readonly guestAuthUrl: string;
  readonly programId: string;
  readonly txlMint: string;
  readonly usdtMint: string;
  readonly rpcUrl: string;
}

export interface DevnetConfigOptions {
  readonly rpcUrl?: string;
}

export function devnetConfig(options: DevnetConfigOptions = {}): TxlineConfig {
  const config: TxlineConfig = {
    network: "devnet",
    apiHost: DEVNET_API_HOST,
    apiBase: DEVNET_API_BASE,
    guestAuthUrl: DEVNET_GUEST_AUTH_URL,
    programId: DEVNET_PROGRAM_ID,
    txlMint: DEVNET_TXL_MINT,
    usdtMint: DEVNET_USDT_MINT,
    rpcUrl: options.rpcUrl ?? DEVNET_RPC_URL,
  };
  validateConfig(config);
  return config;
}

export function withRpcUrl(config: TxlineConfig, rpcUrl: string): TxlineConfig {
  const next = { ...config, rpcUrl };
  validateConfig(next);
  return next;
}

export function validateConfig(config: TxlineConfig): void {
  if (config.network !== "devnet") {
    throw new ConfigError("only TxLINE Devnet is supported by this SDK build");
  }
  if (
    config.apiHost !== DEVNET_API_HOST ||
    config.apiBase !== DEVNET_API_BASE ||
    config.guestAuthUrl !== DEVNET_GUEST_AUTH_URL ||
    config.programId !== DEVNET_PROGRAM_ID ||
    config.txlMint !== DEVNET_TXL_MINT ||
    config.usdtMint !== DEVNET_USDT_MINT
  ) {
    throw new ConfigError(
      "TxLINE Devnet config values must not be mixed with other networks",
    );
  }
  if (config.rpcUrl.trim().length === 0) {
    throw new ConfigError("Solana RPC URL must not be empty");
  }
  if (looksLikeMainnetRpcUrl(config.rpcUrl)) {
    throw new ConfigError(
      "Solana RPC URL must be a Devnet RPC endpoint for this SDK build",
    );
  }
}

export function looksLikeMainnetRpcUrl(rpcUrl: string): boolean {
  return rpcUrl
    .trim()
    .toLowerCase()
    .split(/[^a-z0-9]+/u)
    .some((part) => part === "mainnet" || part === "mainnetbeta");
}
