"""Devnet configuration and RPC guardrails."""

from __future__ import annotations

from dataclasses import dataclass, replace
from enum import Enum

from txline.errors import ConfigError

DEVNET_API_HOST = "https://txline-dev.txodds.com"
DEVNET_API_BASE = "https://txline-dev.txodds.com/api"
DEVNET_GUEST_AUTH_URL = "https://txline-dev.txodds.com/auth/guest/start"
DEVNET_PROGRAM_ID = "6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J"
DEVNET_TXL_MINT = "4Zao8ocPhmMgq7PdsYWyxvqySMGx7xb9cMftPMkEokRG"
DEVNET_USDT_MINT = "ELWTKspHKCnCfCiCiqYw1EDH77k8VCP74dK9qytG2Ujh"
DEVNET_RPC_URL = "https://api.devnet.solana.com"


class Network(str, Enum):
    """Supported TxLINE deployment targets."""

    DEVNET = "devnet"


@dataclass(frozen=True, slots=True)
class TxlineConfig:
    """SDK configuration for TxLINE Devnet."""

    network: Network
    api_host: str
    api_base: str
    guest_auth_url: str
    program_id: str
    txl_mint: str
    usdt_mint: str
    rpc_url: str

    @classmethod
    def devnet(cls) -> TxlineConfig:
        return cls(
            network=Network.DEVNET,
            api_host=DEVNET_API_HOST,
            api_base=DEVNET_API_BASE,
            guest_auth_url=DEVNET_GUEST_AUTH_URL,
            program_id=DEVNET_PROGRAM_ID,
            txl_mint=DEVNET_TXL_MINT,
            usdt_mint=DEVNET_USDT_MINT,
            rpc_url=DEVNET_RPC_URL,
        )

    def with_rpc_url(self, rpc_url: str) -> TxlineConfig:
        return replace(self, rpc_url=rpc_url)

    def validate(self) -> None:
        if self.network != Network.DEVNET:
            raise ConfigError("only TxLINE Devnet is supported by this SDK build")
        if (
            self.api_host != DEVNET_API_HOST
            or self.api_base != DEVNET_API_BASE
            or self.guest_auth_url != DEVNET_GUEST_AUTH_URL
            or self.program_id != DEVNET_PROGRAM_ID
            or self.txl_mint != DEVNET_TXL_MINT
            or self.usdt_mint != DEVNET_USDT_MINT
        ):
            raise ConfigError("TxLINE Devnet config values must not be mixed with other networks")
        if not self.rpc_url.strip():
            raise ConfigError("Solana RPC URL must not be empty")
        if _looks_like_mainnet_rpc_url(self.rpc_url):
            raise ConfigError("Solana RPC URL must be a Devnet RPC endpoint for this SDK build")


def _looks_like_mainnet_rpc_url(rpc_url: str) -> bool:
    parts: list[str] = []
    current: list[str] = []
    for char in rpc_url.strip().lower():
        if char.isalnum():
            current.append(char)
        elif current:
            parts.append("".join(current))
            current.clear()
    if current:
        parts.append("".join(current))
    return any(part in {"mainnet", "mainnetbeta"} for part in parts)
