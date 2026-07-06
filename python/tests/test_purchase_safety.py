from __future__ import annotations

import base64
from dataclasses import replace

import pytest
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from txline.config import TxlineConfig
from txline.purchase import PurchaseQuoteResponse
from txline.solana.instructions import purchase_subscription_token_usdt_instruction
from txline.solana.pda import AccountMeta, DevnetPdas, Instruction, Pubkey
from txline.solana.transaction_safety import (
    LowLevelPurchaseTransactionSafetyConfig,
    PurchaseTransactionSafetyConfig,
    ValidatedPurchaseQuote,
    verify_purchase_transaction_bytes,
    verify_purchase_transaction_bytes_low_level_unchecked_backend_signer,
)


def test_purchase_quote_checked_accepts_synthetic_devnet_transaction() -> None:
    buyer_key = Ed25519PrivateKey.generate()
    backend_key = Ed25519PrivateKey.generate()
    buyer = pubkey_from_private(buyer_key)
    backend = pubkey_from_private(backend_key)
    tx_bytes = signed_purchase_transaction(buyer_key, backend_key, 1_000)
    quote = quote_response(tx_bytes)

    checked = ValidatedPurchaseQuote.new(quote, safety_config(buyer, backend, 1_000))

    assert checked.transaction_bytes() == tx_bytes
    assert checked.safety_report.backend_signer_present
    assert checked.safety_report.txline_purchase_instruction_count == 1


def test_purchase_safety_rejects_amount_mismatch_and_unknown_program() -> None:
    buyer_key = Ed25519PrivateKey.generate()
    backend_key = Ed25519PrivateKey.generate()
    buyer = pubkey_from_private(buyer_key)
    backend = pubkey_from_private(backend_key)
    tx_bytes = signed_purchase_transaction(buyer_key, backend_key, 999)

    with pytest.raises(Exception, match="txline_amount"):
        verify_purchase_transaction_bytes(tx_bytes, safety_config(buyer, backend, 1_000))

    rogue_ix = Instruction(program_id=Pubkey(bytes([200]) * 32), accounts=[], data=b"")
    rogue_tx = signed_purchase_transaction(
        buyer_key, backend_key, 1_000, extra_instructions=[rogue_ix]
    )
    with pytest.raises(Exception, match="unauthorized program"):
        verify_purchase_transaction_bytes(rogue_tx, safety_config(buyer, backend, 1_000))


def test_purchase_safety_rejects_missing_backend_binding_but_low_level_can_inspect() -> None:
    buyer_key = Ed25519PrivateKey.generate()
    backend_key = Ed25519PrivateKey.generate()
    buyer = pubkey_from_private(buyer_key)
    backend = pubkey_from_private(backend_key)
    tx_bytes = signed_purchase_transaction(buyer_key, backend_key, 1_000)
    config = replace(safety_config(buyer, backend, 1_000), expected_backend_signer=None)

    with pytest.raises(Exception, match="requires an expected backend signer"):
        verify_purchase_transaction_bytes(tx_bytes, config)

    report = verify_purchase_transaction_bytes_low_level_unchecked_backend_signer(
        tx_bytes,
        LowLevelPurchaseTransactionSafetyConfig.devnet_unchecked_backend_signer(
            TxlineConfig.devnet(), buyer, 1_000, None
        ),
    )
    assert not report.backend_signer_present
    assert report.txline_purchase_instruction_count == 1


def test_purchase_safety_rejects_address_table_lookups_and_bad_financial_shape() -> None:
    buyer_key = Ed25519PrivateKey.generate()
    backend_key = Ed25519PrivateKey.generate()
    buyer = pubkey_from_private(buyer_key)
    backend = pubkey_from_private(backend_key)

    with pytest.raises(Exception, match="address table lookups"):
        verify_purchase_transaction_bytes(
            signed_purchase_transaction(buyer_key, backend_key, 1_000, versioned_with_lookup=True),
            safety_config(buyer, backend, 1_000),
        )

    bad_quote = PurchaseQuoteResponse("AQ==", 1.0, 0.25, 9.0)
    with pytest.raises(Exception, match="total"):
        bad_quote.validate_financial_shape()


def signed_purchase_transaction(
    buyer_key: Ed25519PrivateKey,
    backend_key: Ed25519PrivateKey,
    amount: int,
    extra_instructions: list[Instruction] | None = None,
    versioned_with_lookup: bool = False,
) -> bytes:
    buyer = pubkey_from_private(buyer_key)
    backend = pubkey_from_private(backend_key)
    pdas = DevnetPdas.new()
    purchase_ix = purchase_subscription_token_usdt_instruction(
        pdas.program_id, pdas.purchase_accounts(buyer, backend), amount
    )
    instructions = [purchase_ix, *(extra_instructions or [])]
    message = compile_message(instructions, buyer, versioned_with_lookup)
    signatures = [buyer_key.sign(message), backend_key.sign(message)]
    return shortvec(len(signatures)) + b"".join(signatures) + message


def compile_message(
    instructions: list[Instruction], payer: Pubkey, versioned_with_lookup: bool
) -> bytes:
    metas: dict[Pubkey, AccountMeta] = {payer: AccountMeta.writable(payer, True)}
    for ix in instructions:
        for meta in ix.accounts:
            existing = metas.get(meta.pubkey)
            metas[meta.pubkey] = AccountMeta(
                meta.pubkey,
                is_signer=meta.is_signer or (existing.is_signer if existing else False),
                is_writable=meta.is_writable or (existing.is_writable if existing else False),
            )
        metas.setdefault(ix.program_id, AccountMeta.readonly(ix.program_id))

    def sort_key(meta: AccountMeta) -> tuple[int, int]:
        if meta.pubkey == payer:
            return (0, 0)
        if meta.is_signer and meta.is_writable:
            return (0, 1)
        if meta.is_signer:
            return (1, 0)
        if meta.is_writable:
            return (2, 0)
        return (3, 0)

    ordered = sorted(metas.values(), key=sort_key)
    account_keys = [meta.pubkey for meta in ordered]
    num_required_signatures = sum(1 for meta in ordered if meta.is_signer)
    num_readonly_signed = sum(1 for meta in ordered if meta.is_signer and not meta.is_writable)
    num_readonly_unsigned = sum(
        1 for meta in ordered if not meta.is_signer and not meta.is_writable
    )

    out = bytearray()
    if versioned_with_lookup:
        out.append(0x80)
    out.extend(bytes([num_required_signatures, num_readonly_signed, num_readonly_unsigned]))
    out.extend(shortvec(len(account_keys)))
    for key in account_keys:
        out.extend(key.as_bytes())
    out.extend(bytes([42]) * 32)
    out.extend(shortvec(len(instructions)))
    for ix in instructions:
        out.append(account_keys.index(ix.program_id))
        out.extend(shortvec(len(ix.accounts)))
        out.extend(account_keys.index(meta.pubkey) for meta in ix.accounts)
        out.extend(shortvec(len(ix.data)))
        out.extend(ix.data)
    if versioned_with_lookup:
        out.extend(shortvec(1))
        out.extend(bytes([99]) * 32)
        out.extend(shortvec(0))
        out.extend(shortvec(1))
        out.append(0)
    return bytes(out)


def safety_config(buyer: Pubkey, backend: Pubkey, amount: int) -> PurchaseTransactionSafetyConfig:
    return PurchaseTransactionSafetyConfig.devnet(TxlineConfig.devnet(), buyer, amount, backend)


def quote_response(transaction_bytes: bytes) -> PurchaseQuoteResponse:
    return PurchaseQuoteResponse(base64.b64encode(transaction_bytes).decode(), 1.0, 0.25, 1.25)


def pubkey_from_private(private_key: Ed25519PrivateKey) -> Pubkey:
    public_bytes = private_key.public_key().public_bytes_raw()
    return Pubkey(public_bytes)


def shortvec(value: int) -> bytes:
    out = bytearray()
    while True:
        elem = value & 0x7F
        value >>= 7
        if value:
            elem |= 0x80
        out.append(elem)
        if not value:
            return bytes(out)
