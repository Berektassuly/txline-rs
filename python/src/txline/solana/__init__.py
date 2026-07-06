"""Devnet Solana helpers."""

from __future__ import annotations

from dataclasses import dataclass

from txline.config import TxlineConfig
from txline.solana.instructions import (
    audit_trade_result_instruction,
    claim_batch_legacy_instruction,
    claim_via_resolution_instruction,
    close_intent_instruction,
    create_intent_instruction,
    create_token_2022_associated_token_account_instruction,
    create_trade_instruction,
    execute_match_instruction,
    purchase_subscription_token_usdt_instruction,
    refund_batch_instruction,
    request_devnet_faucet_instruction,
    settle_matched_trade_instruction,
    settle_trade_instruction,
    subscribe_instruction,
    validate_fixture_batch_instruction,
    validate_fixture_instruction,
    validate_odds_instruction,
    validate_stat_instruction,
    validate_stat_v2_instruction,
)
from txline.solana.pda import DevnetPdas, Instruction, Pubkey, ensure_pubkey


@dataclass(frozen=True, slots=True)
class SolanaClient:
    config: TxlineConfig

    def program_id(self) -> Pubkey:
        return ensure_pubkey(self.config.program_id)

    def pdas(self) -> DevnetPdas:
        return DevnetPdas.new()

    def build_subscribe_instruction(
        self, user: str | Pubkey, service_level_id: int, weeks: int
    ) -> Instruction:
        return subscribe_instruction(
            self.program_id(),
            self.pdas().subscribe_accounts(ensure_pubkey(user)),
            service_level_id,
            weeks,
        )

    def build_purchase_subscription_token_usdt_instruction(
        self, buyer: str | Pubkey, backend_admin: str | Pubkey, txline_amount: int
    ) -> Instruction:
        return purchase_subscription_token_usdt_instruction(
            self.program_id(),
            self.pdas().purchase_accounts(ensure_pubkey(buyer), ensure_pubkey(backend_admin)),
            txline_amount,
        )


__all__ = [
    "DevnetPdas",
    "Pubkey",
    "SolanaClient",
    "audit_trade_result_instruction",
    "claim_batch_legacy_instruction",
    "claim_via_resolution_instruction",
    "close_intent_instruction",
    "create_intent_instruction",
    "create_token_2022_associated_token_account_instruction",
    "create_trade_instruction",
    "ensure_pubkey",
    "execute_match_instruction",
    "purchase_subscription_token_usdt_instruction",
    "refund_batch_instruction",
    "request_devnet_faucet_instruction",
    "settle_matched_trade_instruction",
    "settle_trade_instruction",
    "subscribe_instruction",
    "validate_fixture_batch_instruction",
    "validate_fixture_instruction",
    "validate_odds_instruction",
    "validate_stat_instruction",
    "validate_stat_v2_instruction",
]
