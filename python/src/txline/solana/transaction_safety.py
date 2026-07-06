"""Conservative purchase quote transaction safety checks."""

from __future__ import annotations

from dataclasses import dataclass

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

from txline.config import TxlineConfig
from txline.errors import SolanaError
from txline.purchase import PurchaseQuoteResponse
from txline.solana.instructions import PURCHASE_SUBSCRIPTION_TOKEN_USDT_DISCRIMINATOR
from txline.solana.pda import (
    ASSOCIATED_TOKEN_PROGRAM_ID,
    COMPUTE_BUDGET_PROGRAM_ID,
    LEGACY_TOKEN_PROGRAM_ID,
    SYSTEM_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    DevnetPdas,
    Pubkey,
    ensure_pubkey,
)


@dataclass(frozen=True, slots=True)
class PurchaseTransactionSafetyConfig:
    txline_program_id: Pubkey
    expected_buyer: Pubkey
    expected_txline_amount: int
    expected_backend_signer: Pubkey | None

    @classmethod
    def devnet(
        cls,
        config: TxlineConfig,
        expected_buyer: str | Pubkey,
        expected_txline_amount: int,
        expected_backend_signer: str | Pubkey,
    ) -> PurchaseTransactionSafetyConfig:
        return cls(
            txline_program_id=ensure_pubkey(config.program_id),
            expected_buyer=ensure_pubkey(expected_buyer),
            expected_txline_amount=expected_txline_amount,
            expected_backend_signer=ensure_pubkey(expected_backend_signer),
        )

    def low_level_config(self) -> LowLevelPurchaseTransactionSafetyConfig:
        if self.expected_backend_signer is None:
            raise SolanaError("safe purchase validation requires an expected backend signer")
        return LowLevelPurchaseTransactionSafetyConfig(
            txline_program_id=self.txline_program_id,
            expected_buyer=self.expected_buyer,
            expected_txline_amount=self.expected_txline_amount,
            expected_backend_signer=self.expected_backend_signer,
        )


@dataclass(frozen=True, slots=True)
class LowLevelPurchaseTransactionSafetyConfig:
    txline_program_id: Pubkey
    expected_buyer: Pubkey
    expected_txline_amount: int
    expected_backend_signer: Pubkey | None

    @classmethod
    def devnet_unchecked_backend_signer(
        cls,
        config: TxlineConfig,
        expected_buyer: str | Pubkey,
        expected_txline_amount: int,
        expected_backend_signer: str | Pubkey | None,
    ) -> LowLevelPurchaseTransactionSafetyConfig:
        return cls(
            txline_program_id=ensure_pubkey(config.program_id),
            expected_buyer=ensure_pubkey(expected_buyer),
            expected_txline_amount=expected_txline_amount,
            expected_backend_signer=(
                ensure_pubkey(expected_backend_signer)
                if expected_backend_signer is not None
                else None
            ),
        )


@dataclass(frozen=True, slots=True)
class PurchaseTransactionSafetyReport:
    fee_payer: Pubkey
    invoked_programs: list[Pubkey]
    txline_purchase_instruction_count: int
    backend_signer_present: bool


@dataclass(frozen=True, slots=True)
class ValidatedPurchaseQuote:
    quote: PurchaseQuoteResponse
    safety_report: PurchaseTransactionSafetyReport
    _transaction_bytes: bytes

    @classmethod
    def new(
        cls, quote: PurchaseQuoteResponse, config: PurchaseTransactionSafetyConfig
    ) -> ValidatedPurchaseQuote:
        quote.validate_financial_shape()
        transaction_bytes = quote.raw_transaction_bytes_unchecked()
        safety_report = verify_purchase_transaction_bytes(transaction_bytes, config)
        return cls(quote=quote, safety_report=safety_report, _transaction_bytes=transaction_bytes)

    def transaction_bytes(self) -> bytes:
        return self._transaction_bytes


@dataclass(frozen=True, slots=True)
class CompiledInstruction:
    program_id_index: int
    accounts: list[int]
    data: bytes


@dataclass(frozen=True, slots=True)
class DecodedMessage:
    version: int | None
    message_bytes: bytes
    num_required_signatures: int
    account_keys: list[Pubkey]
    instructions: list[CompiledInstruction]
    address_table_lookup_count: int

    def is_signer(self, index: int) -> bool:
        return 0 <= index < self.num_required_signatures


@dataclass(frozen=True, slots=True)
class DecodedTransaction:
    signatures: list[bytes]
    message: DecodedMessage


def verify_purchase_transaction_base64(
    transaction_base64: str, config: PurchaseTransactionSafetyConfig
) -> PurchaseTransactionSafetyReport:
    import base64

    return verify_purchase_transaction_bytes(base64.b64decode(transaction_base64), config)


def verify_purchase_transaction_bytes(
    transaction_bytes: bytes, config: PurchaseTransactionSafetyConfig
) -> PurchaseTransactionSafetyReport:
    return verify_purchase_transaction(decode_versioned_transaction(transaction_bytes), config)


def verify_purchase_transaction_bytes_low_level_unchecked_backend_signer(
    transaction_bytes: bytes, config: LowLevelPurchaseTransactionSafetyConfig
) -> PurchaseTransactionSafetyReport:
    return verify_purchase_transaction_low_level_unchecked_backend_signer(
        decode_versioned_transaction(transaction_bytes), config
    )


def verify_purchase_transaction(
    transaction: DecodedTransaction, config: PurchaseTransactionSafetyConfig
) -> PurchaseTransactionSafetyReport:
    return verify_purchase_transaction_low_level_unchecked_backend_signer(
        transaction, config.low_level_config()
    )


def verify_purchase_transaction_low_level_unchecked_backend_signer(
    transaction: DecodedTransaction, config: LowLevelPurchaseTransactionSafetyConfig
) -> PurchaseTransactionSafetyReport:
    if transaction.message.address_table_lookup_count:
        raise SolanaError(
            "purchase quote uses address table lookups; "
            "SDK cannot audit dynamically loaded accounts safely"
        )
    account_keys = transaction.message.account_keys
    if not account_keys:
        raise SolanaError("purchase transaction has no fee payer")
    fee_payer = account_keys[0]
    if fee_payer != config.expected_buyer:
        raise SolanaError("purchase transaction fee payer is not the expected buyer")

    backend_signer_present = False
    if config.expected_backend_signer is not None:
        backend_signer_present = _signer_signature_present(
            transaction, config.expected_backend_signer
        )
        if not backend_signer_present:
            raise SolanaError(
                "purchase transaction is missing the expected backend signer signature"
            )

    allowed_programs = set(_allowed_purchase_program_pubkeys(config.txline_program_id))
    invoked_programs: list[Pubkey] = []
    purchase_instruction_count = 0
    for instruction in transaction.message.instructions:
        try:
            program_id = account_keys[instruction.program_id_index]
        except IndexError as exc:
            raise SolanaError("purchase instruction program index is invalid") from exc
        if program_id not in allowed_programs:
            raise SolanaError(f"purchase transaction invokes unauthorized program {program_id}")
        if program_id not in invoked_programs:
            invoked_programs.append(program_id)
        _reject_unexpected_buyer_signer(
            transaction, program_id, config.txline_program_id, instruction.accounts
        )
        if program_id == config.txline_program_id:
            purchase_instruction_count += 1
            _verify_purchase_instruction_data(instruction.data, config)
            _verify_purchase_instruction_accounts(account_keys, instruction.accounts, config)

    if purchase_instruction_count != 1:
        raise SolanaError(
            "purchase transaction must contain exactly one TxLINE purchase instruction, "
            f"found {purchase_instruction_count}"
        )

    return PurchaseTransactionSafetyReport(
        fee_payer=fee_payer,
        invoked_programs=invoked_programs,
        txline_purchase_instruction_count=purchase_instruction_count,
        backend_signer_present=backend_signer_present,
    )


def allowed_purchase_programs(txline_program_id: str) -> list[str]:
    return [
        txline_program_id,
        COMPUTE_BUDGET_PROGRAM_ID,
        SYSTEM_PROGRAM_ID,
        LEGACY_TOKEN_PROGRAM_ID,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
    ]


def decode_versioned_transaction(transaction_bytes: bytes) -> DecodedTransaction:
    if not transaction_bytes:
        raise SolanaError("purchase quote transaction decoded to an empty byte buffer")
    reader = _Reader(transaction_bytes)
    signature_count = reader.shortvec()
    signatures = [reader.bytes(64) for _ in range(signature_count)]
    message_start = reader.offset
    first = reader.u8()
    if first & 0x80:
        version = first & 0x7F
        if version != 0:
            raise SolanaError(f"unsupported versioned transaction message v{version}")
        num_required_signatures = reader.u8()
        reader.u8()
        reader.u8()
    else:
        version = None
        num_required_signatures = first
        reader.u8()
        reader.u8()
    account_count = reader.shortvec()
    account_keys = [Pubkey(reader.bytes(32)) for _ in range(account_count)]
    reader.bytes(32)
    instruction_count = reader.shortvec()
    instructions: list[CompiledInstruction] = []
    for _ in range(instruction_count):
        program_id_index = reader.u8()
        accounts = list(reader.bytes(reader.shortvec()))
        data = reader.bytes(reader.shortvec())
        instructions.append(
            CompiledInstruction(
                program_id_index=program_id_index,
                accounts=accounts,
                data=data,
            )
        )
    address_table_lookup_count = 0
    if version == 0:
        address_table_lookup_count = reader.shortvec()
        for _ in range(address_table_lookup_count):
            reader.bytes(32)
            reader.bytes(reader.shortvec())
            reader.bytes(reader.shortvec())
    if reader.offset != len(transaction_bytes):
        raise SolanaError("purchase transaction has unexpected trailing bytes")
    return DecodedTransaction(
        signatures=signatures,
        message=DecodedMessage(
            version=version,
            message_bytes=transaction_bytes[message_start:],
            num_required_signatures=num_required_signatures,
            account_keys=account_keys,
            instructions=instructions,
            address_table_lookup_count=address_table_lookup_count,
        ),
    )


def _allowed_purchase_program_pubkeys(txline_program_id: Pubkey) -> list[Pubkey]:
    return [ensure_pubkey(program) for program in allowed_purchase_programs(str(txline_program_id))]


def _signer_signature_present(transaction: DecodedTransaction, signer: Pubkey) -> bool:
    try:
        signer_index = transaction.message.account_keys.index(signer)
    except ValueError as exc:
        raise SolanaError("expected backend signer is not present in transaction accounts") from exc
    if not transaction.message.is_signer(signer_index):
        raise SolanaError("expected backend signer account is not marked as a signer")
    signature = (
        transaction.signatures[signer_index] if signer_index < len(transaction.signatures) else b""
    )
    if signature == b"\0" * 64:
        return False
    try:
        Ed25519PublicKey.from_public_bytes(signer.as_bytes()).verify(
            signature, transaction.message.message_bytes
        )
    except InvalidSignature as exc:
        raise SolanaError("expected backend signer signature does not verify") from exc
    return True


def _reject_unexpected_buyer_signer(
    transaction: DecodedTransaction,
    program_id: Pubkey,
    txline_program_id: Pubkey,
    instruction_accounts: list[int],
) -> None:
    buyer_index = 0
    if buyer_index in instruction_accounts and transaction.message.is_signer(buyer_index):
        ata_program = ensure_pubkey(ASSOCIATED_TOKEN_PROGRAM_ID)
        if program_id not in {txline_program_id, ata_program}:
            raise SolanaError(
                f"buyer wallet is requested as signer for unauthorized program {program_id}"
            )


def _verify_purchase_instruction_data(
    data: bytes, config: LowLevelPurchaseTransactionSafetyConfig
) -> None:
    if len(data) != 16:
        raise SolanaError(f"purchase instruction data length is {len(data)}, expected 16")
    if data[:8] != PURCHASE_SUBSCRIPTION_TOKEN_USDT_DISCRIMINATOR:
        raise SolanaError("TxLINE instruction is not purchase_subscription_token_usdt")
    amount = int.from_bytes(data[8:16], "little")
    if amount != config.expected_txline_amount:
        raise SolanaError(
            f"purchase txline_amount {amount} does not match expected "
            f"{config.expected_txline_amount}"
        )


def _verify_purchase_instruction_accounts(
    account_keys: list[Pubkey],
    instruction_accounts: list[int],
    config: LowLevelPurchaseTransactionSafetyConfig,
) -> None:
    if len(instruction_accounts) != 14:
        raise SolanaError(
            f"purchase instruction account count is {len(instruction_accounts)}, expected 14"
        )
    pdas = DevnetPdas.new()
    expected_accounts = [
        config.expected_buyer,
        config.expected_backend_signer,
        pdas.usdt_mint,
        pdas.user_usdt_ata(config.expected_buyer).address,
        pdas.usdt_treasury_vault_ata().address,
        pdas.usdt_treasury().address,
        pdas.txl_mint,
        pdas.token_treasury_vault_ata().address,
        pdas.token_treasury_v2().address,
        pdas.user_txl_ata(config.expected_buyer).address,
        ensure_pubkey(LEGACY_TOKEN_PROGRAM_ID),
        ensure_pubkey(TOKEN_2022_PROGRAM_ID),
        ensure_pubkey(SYSTEM_PROGRAM_ID),
        ensure_pubkey(ASSOCIATED_TOKEN_PROGRAM_ID),
    ]
    for position, expected in enumerate(expected_accounts):
        actual_index = instruction_accounts[position]
        try:
            actual = account_keys[actual_index]
        except IndexError as exc:
            raise SolanaError(
                f"purchase instruction account index {actual_index} is invalid"
            ) from exc
        if expected is not None and actual != expected:
            raise SolanaError(
                f"purchase instruction account {position} is {actual}, expected {expected}"
            )


class _Reader:
    def __init__(self, data: bytes) -> None:
        self._data = data
        self.offset = 0

    def u8(self) -> int:
        return self.bytes(1)[0]

    def bytes(self, length: int) -> bytes:
        if self.offset + length > len(self._data):
            raise SolanaError("purchase transaction ended unexpectedly")
        value = self._data[self.offset : self.offset + length]
        self.offset += length
        return value

    def shortvec(self) -> int:
        value = 0
        shift = 0
        while True:
            byte = self.u8()
            value |= (byte & 0x7F) << shift
            if byte & 0x80 == 0:
                return value
            shift += 7
            if shift > 28:
                raise SolanaError("invalid Solana compact-u16 length")
