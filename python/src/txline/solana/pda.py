"""Solana pubkey, PDA, and account meta helpers."""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from typing import Any

from txline.config import DEVNET_PROGRAM_ID, DEVNET_TXL_MINT, DEVNET_USDT_MINT
from txline.errors import SolanaError

TOKEN_2022_PROGRAM_ID = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
LEGACY_TOKEN_PROGRAM_ID = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
ASSOCIATED_TOKEN_PROGRAM_ID = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
SYSTEM_PROGRAM_ID = "11111111111111111111111111111111"
COMPUTE_BUDGET_PROGRAM_ID = "ComputeBudget111111111111111111111111111111"

_B58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
_B58_INDEX = {char: idx for idx, char in enumerate(_B58_ALPHABET)}
_PDA_MARKER = b"ProgramDerivedAddress"
_ED25519_P = 2**255 - 19
_ED25519_D = (-121665 * pow(121666, _ED25519_P - 2, _ED25519_P)) % _ED25519_P
_ED25519_I = pow(2, (_ED25519_P - 1) // 4, _ED25519_P)


@dataclass(frozen=True, slots=True)
class Pubkey:
    value: bytes

    def __init__(self, value: bytes | bytearray | list[int] | tuple[int, ...]) -> None:
        raw = bytes(value)
        if len(raw) != 32:
            raise SolanaError(f"pubkey must be 32 bytes, got {len(raw)}")
        object.__setattr__(self, "value", raw)

    @classmethod
    def from_string(cls, value: str) -> Pubkey:
        try:
            decoded = b58decode(value)
        except ValueError as exc:
            raise SolanaError(f"invalid pubkey {value}: {exc}") from exc
        return cls(decoded)

    @classmethod
    def from_base(cls, byte: int) -> Pubkey:
        return cls(bytes([byte]) * 32)

    def as_bytes(self) -> bytes:
        return self.value

    def __bytes__(self) -> bytes:
        return self.value

    def __str__(self) -> str:
        return b58encode(self.value)

    def __repr__(self) -> str:
        return f"Pubkey({str(self)})"


@dataclass(frozen=True, slots=True)
class Pda:
    address: Pubkey
    bump: int


@dataclass(frozen=True, slots=True)
class AccountMeta:
    pubkey: Pubkey
    is_signer: bool
    is_writable: bool

    @classmethod
    def writable(cls, pubkey: str | Pubkey, is_signer: bool = False) -> AccountMeta:
        return cls(ensure_pubkey(pubkey), is_signer=is_signer, is_writable=True)

    @classmethod
    def readonly(cls, pubkey: str | Pubkey, is_signer: bool = False) -> AccountMeta:
        return cls(ensure_pubkey(pubkey), is_signer=is_signer, is_writable=False)


@dataclass(frozen=True, slots=True)
class Instruction:
    program_id: Pubkey
    accounts: list[AccountMeta]
    data: bytes


@dataclass(frozen=True, slots=True)
class DevnetPdas:
    program_id: Pubkey
    txl_mint: Pubkey
    usdt_mint: Pubkey

    @classmethod
    def new(cls) -> DevnetPdas:
        return cls(
            program_id=Pubkey.from_string(DEVNET_PROGRAM_ID),
            txl_mint=Pubkey.from_string(DEVNET_TXL_MINT),
            usdt_mint=Pubkey.from_string(DEVNET_USDT_MINT),
        )

    def pricing_matrix(self) -> Pda:
        return find_program_address([b"pricing_matrix"], self.program_id)

    def token_treasury_v2(self) -> Pda:
        return find_program_address([b"token_treasury_v2"], self.program_id)

    def usdt_treasury(self) -> Pda:
        return find_program_address([b"usdt_treasury"], self.program_id)

    def token_treasury_vault_ata(self) -> Pda:
        return token_2022_associated_token_address(self.token_treasury_v2().address, self.txl_mint)

    def usdt_treasury_vault_ata(self) -> Pda:
        return token_2022_associated_token_address(self.usdt_treasury().address, self.usdt_mint)

    def user_txl_ata(self, user: Pubkey) -> Pda:
        return token_2022_associated_token_address(user, self.txl_mint)

    def user_usdt_ata(self, user: Pubkey) -> Pda:
        return token_2022_associated_token_address(user, self.usdt_mint)

    def daily_scores_roots(self, epoch_day: int) -> Pda:
        return find_program_address(
            [b"daily_scores_roots", int(epoch_day).to_bytes(2, "little")], self.program_id
        )

    def daily_batch_roots(self, epoch_day: int) -> Pda:
        return find_program_address(
            [b"daily_batch_roots", int(epoch_day).to_bytes(2, "little")], self.program_id
        )

    def daily_odds_merkle_roots(self, epoch_day: int) -> Pda:
        return self.daily_batch_roots(epoch_day)

    def ten_daily_fixtures_roots(self, epoch_day: int) -> Pda:
        aligned = int(epoch_day) - (int(epoch_day) % 10)
        return find_program_address(
            [b"ten_daily_fixtures_roots", aligned.to_bytes(2, "little")], self.program_id
        )

    def subscribe_accounts(self, user: Pubkey) -> Any:
        from txline.solana.instructions import SubscribeAccounts

        return SubscribeAccounts(
            user=user,
            pricing_matrix=self.pricing_matrix().address,
            token_mint=self.txl_mint,
            user_token_account=self.user_txl_ata(user).address,
            token_treasury_vault=self.token_treasury_vault_ata().address,
            token_treasury_pda=self.token_treasury_v2().address,
            token_program=Pubkey.from_string(TOKEN_2022_PROGRAM_ID),
            system_program=Pubkey.from_string(SYSTEM_PROGRAM_ID),
            associated_token_program=Pubkey.from_string(ASSOCIATED_TOKEN_PROGRAM_ID),
        )

    def purchase_accounts(self, buyer: Pubkey, backend_admin: Pubkey) -> Any:
        from txline.solana.instructions import PurchaseSubscriptionTokenUsdtAccounts

        return PurchaseSubscriptionTokenUsdtAccounts(
            buyer=buyer,
            backend_admin=backend_admin,
            usdt_mint=self.usdt_mint,
            buyer_usdt_account=self.user_usdt_ata(buyer).address,
            usdt_treasury_vault=self.usdt_treasury_vault_ata().address,
            usdt_treasury_pda=self.usdt_treasury().address,
            subscription_token_mint=self.txl_mint,
            token_treasury_vault=self.token_treasury_vault_ata().address,
            token_treasury_pda=self.token_treasury_v2().address,
            buyer_token_account=self.user_txl_ata(buyer).address,
            token_program=Pubkey.from_string(LEGACY_TOKEN_PROGRAM_ID),
            token_2022_program=Pubkey.from_string(TOKEN_2022_PROGRAM_ID),
            system_program=Pubkey.from_string(SYSTEM_PROGRAM_ID),
            associated_token_program=Pubkey.from_string(ASSOCIATED_TOKEN_PROGRAM_ID),
        )


def ensure_pubkey(value: str | Pubkey | bytes | bytearray) -> Pubkey:
    if isinstance(value, Pubkey):
        return value
    if isinstance(value, str):
        return Pubkey.from_string(value)
    return Pubkey(value)


def token_2022_associated_token_address(owner: Pubkey, mint: Pubkey) -> Pda:
    token_program = Pubkey.from_string(TOKEN_2022_PROGRAM_ID)
    associated_program = Pubkey.from_string(ASSOCIATED_TOKEN_PROGRAM_ID)
    return find_program_address(
        [owner.as_bytes(), token_program.as_bytes(), mint.as_bytes()], associated_program
    )


def find_program_address(seeds: list[bytes], program_id: Pubkey) -> Pda:
    for bump in range(255, -1, -1):
        try:
            address = create_program_address([*seeds, bytes([bump])], program_id)
            return Pda(address=address, bump=bump)
        except SolanaError:
            continue
    raise SolanaError("could not find a valid program address")


def create_program_address(seeds: list[bytes], program_id: Pubkey) -> Pubkey:
    if len(seeds) > 16:
        raise SolanaError("max seed count exceeded")
    for seed in seeds:
        if len(seed) > 32:
            raise SolanaError("max seed length exceeded")
    digest = hashlib.sha256(b"".join(seeds) + program_id.as_bytes() + _PDA_MARKER).digest()
    if _is_on_ed25519_curve(digest):
        raise SolanaError("derived address falls on the ed25519 curve")
    return Pubkey(digest)


def b58encode(data: bytes) -> str:
    value = int.from_bytes(data, "big")
    chars = []
    while value:
        value, remainder = divmod(value, 58)
        chars.append(_B58_ALPHABET[remainder])
    leading = len(data) - len(data.lstrip(b"\0"))
    encoded = "".join(reversed(chars))
    return "1" * leading + encoded


def b58decode(value: str) -> bytes:
    number = 0
    for char in value:
        try:
            digit = _B58_INDEX[char]
        except KeyError as exc:
            raise ValueError(f"invalid base58 character {char!r}") from exc
        number = number * 58 + digit
    raw = number.to_bytes((number.bit_length() + 7) // 8, "big") if number else b""
    leading = len(value) - len(value.lstrip("1"))
    return b"\0" * leading + raw


def _is_on_ed25519_curve(value: bytes) -> bool:
    y = int.from_bytes(value, "little") & ((1 << 255) - 1)
    if y >= _ED25519_P:
        return False
    y2 = (y * y) % _ED25519_P
    u = (y2 - 1) % _ED25519_P
    v = (_ED25519_D * y2 + 1) % _ED25519_P
    try:
        x2 = (u * pow(v, _ED25519_P - 2, _ED25519_P)) % _ED25519_P
    except ValueError:
        return False
    if x2 == 0:
        return True
    x = pow(x2, (_ED25519_P + 3) // 8, _ED25519_P)
    if (x * x - x2) % _ED25519_P != 0:
        x = (x * _ED25519_I) % _ED25519_P
    return (x * x - x2) % _ED25519_P == 0
