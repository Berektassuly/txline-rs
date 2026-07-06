"""Purchase quote REST DTOs and checked helpers."""

from __future__ import annotations

import base64
from dataclasses import dataclass
from typing import Any

from txline.errors import SolanaError

MAX_QUOTE_TXLINE_AMOUNT = 100_000_000


@dataclass(frozen=True, slots=True)
class PurchaseQuoteResponse:
    transaction_base64: str
    base_usdt_cost: float
    fee_usdt_amount: float
    total_usdt_charged: float

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> PurchaseQuoteResponse:
        return cls(
            transaction_base64=str(data["transactionBase64"]),
            base_usdt_cost=float(data["baseUsdtCost"]),
            fee_usdt_amount=float(data["feeUsdtAmount"]),
            total_usdt_charged=float(data["totalUsdtCharged"]),
        )

    def raw_transaction_bytes_unchecked(self) -> bytes:
        decoded = base64.b64decode(self.transaction_base64)
        if not decoded:
            raise SolanaError("purchase quote transaction decoded to an empty byte buffer")
        return decoded

    def validate_financial_shape(self) -> None:
        if self.base_usdt_cost < 0 or self.fee_usdt_amount < 0 or self.total_usdt_charged < 0:
            raise SolanaError("purchase quote contains negative USDT amounts")
        expected = self.base_usdt_cost + self.fee_usdt_amount
        if abs(expected - self.total_usdt_charged) > 0.000_001:
            raise SolanaError("purchase quote total does not equal base cost plus fee")

    def validated_transaction_bytes(self, config: Any) -> bytes:
        from txline.solana.transaction_safety import verify_purchase_transaction_bytes

        self.validate_financial_shape()
        transaction_bytes = self.raw_transaction_bytes_unchecked()
        verify_purchase_transaction_bytes(transaction_bytes, config)
        return transaction_bytes


def validate_quote_amount(txline_amount: int) -> None:
    if txline_amount <= 0 or txline_amount > MAX_QUOTE_TXLINE_AMOUNT:
        raise SolanaError(f"txline_amount must be 1..={MAX_QUOTE_TXLINE_AMOUNT}")
