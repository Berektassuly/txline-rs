"""Anchor/Borsh instruction builders for the TxLINE Devnet program."""

from __future__ import annotations

import struct
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from typing import TypeVar

from txline.errors import InvalidInputError, ValidationError
from txline.fixtures import (
    BatchMetadata,
    Fixture,
    FixtureBatchSummary,
    FixtureBatchValidation,
    FixtureValidation,
)
from txline.odds import OddsBatchSummary, OddsPayload, OddsValidation
from txline.purchase import validate_quote_amount
from txline.solana.pda import (
    ASSOCIATED_TOKEN_PROGRAM_ID,
    SYSTEM_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    AccountMeta,
    Instruction,
    Pubkey,
    ensure_pubkey,
)
from txline.validation.legacy import (
    FixtureSummaryInput,
    ScoresStatValidation,
    ScoreStat,
    StatTermInput,
)
from txline.validation.proof import ProofNode
from txline.validation.strategy import (
    BinaryExpression,
    BinaryPredicate,
    Comparison,
    GeometricTarget,
    NDimensionalStrategy,
    SinglePredicate,
    TraderPredicate,
)
from txline.validation.v2 import StatLeafInput, StatValidationInput

_T = TypeVar("_T")

SUBSCRIBE_DISCRIMINATOR = bytes([254, 28, 191, 138, 156, 179, 183, 53])
REQUEST_DEVNET_FAUCET_DISCRIMINATOR = bytes([49, 178, 104, 8, 23, 120, 186, 21])
PURCHASE_SUBSCRIPTION_TOKEN_USDT_DISCRIMINATOR = bytes([198, 251, 223, 9, 31, 184, 166, 188])
VALIDATE_FIXTURE_DISCRIMINATOR = bytes([231, 129, 218, 86, 223, 114, 21, 126])
VALIDATE_FIXTURE_BATCH_DISCRIMINATOR = bytes([85, 223, 204, 7, 4, 87, 157, 1])
VALIDATE_ODDS_DISCRIMINATOR = bytes([192, 19, 91, 138, 104, 100, 212, 86])
VALIDATE_STAT_DISCRIMINATOR = bytes([107, 197, 232, 90, 191, 136, 105, 185])
VALIDATE_STAT_V2_DISCRIMINATOR = bytes([208, 215, 194, 214, 241, 71, 246, 178])
CREATE_INTENT_DISCRIMINATOR = bytes([216, 214, 79, 121, 23, 194, 96, 104])
CREATE_TRADE_DISCRIMINATOR = bytes([183, 82, 24, 245, 248, 30, 204, 246])
EXECUTE_MATCH_DISCRIMINATOR = bytes([76, 47, 91, 223, 20, 10, 147, 232])
CLOSE_INTENT_DISCRIMINATOR = bytes([112, 245, 154, 249, 57, 126, 54, 122])
SETTLE_TRADE_DISCRIMINATOR = bytes([252, 176, 98, 248, 73, 123, 8, 157])
SETTLE_MATCHED_TRADE_DISCRIMINATOR = bytes([191, 233, 149, 116, 32, 239, 18, 65])
CLAIM_VIA_RESOLUTION_DISCRIMINATOR = bytes([98, 206, 250, 87, 151, 135, 162, 181])
CLAIM_BATCH_LEGACY_DISCRIMINATOR = bytes([254, 101, 89, 255, 169, 75, 207, 66])
REFUND_BATCH_DISCRIMINATOR = bytes([227, 54, 194, 2, 78, 8, 104, 29])
AUDIT_TRADE_RESULT_DISCRIMINATOR = bytes([50, 242, 243, 5, 209, 75, 76, 91])


@dataclass(frozen=True, slots=True)
class SubscribeAccounts:
    user: Pubkey
    pricing_matrix: Pubkey
    token_mint: Pubkey
    user_token_account: Pubkey
    token_treasury_vault: Pubkey
    token_treasury_pda: Pubkey
    token_program: Pubkey
    system_program: Pubkey
    associated_token_program: Pubkey


@dataclass(frozen=True, slots=True)
class RequestDevnetFaucetAccounts:
    user: Pubkey
    faucet_tracker: Pubkey
    usdt_mint: Pubkey
    user_usdt_ata: Pubkey
    usdt_treasury_pda: Pubkey
    token_program: Pubkey
    associated_token_program: Pubkey
    system_program: Pubkey


@dataclass(frozen=True, slots=True)
class PurchaseSubscriptionTokenUsdtAccounts:
    buyer: Pubkey
    backend_admin: Pubkey
    usdt_mint: Pubkey
    buyer_usdt_account: Pubkey
    usdt_treasury_vault: Pubkey
    usdt_treasury_pda: Pubkey
    subscription_token_mint: Pubkey
    token_treasury_vault: Pubkey
    token_treasury_pda: Pubkey
    buyer_token_account: Pubkey
    token_program: Pubkey
    token_2022_program: Pubkey
    system_program: Pubkey
    associated_token_program: Pubkey


def subscribe_instruction(
    program_id: Pubkey, accounts: SubscribeAccounts, service_level_id: int, weeks: int
) -> Instruction:
    validate_subscription_weeks(weeks)
    data = bytearray(SUBSCRIBE_DISCRIMINATOR)
    _put_u16(data, service_level_id)
    _put_u8(data, weeks)
    return Instruction(
        program_id=program_id,
        accounts=[
            AccountMeta.writable(accounts.user, True),
            AccountMeta.readonly(accounts.pricing_matrix),
            AccountMeta.readonly(accounts.token_mint),
            AccountMeta.writable(accounts.user_token_account),
            AccountMeta.writable(accounts.token_treasury_vault),
            AccountMeta.readonly(accounts.token_treasury_pda),
            AccountMeta.readonly(accounts.token_program),
            AccountMeta.readonly(accounts.system_program),
            AccountMeta.readonly(accounts.associated_token_program),
        ],
        data=bytes(data),
    )


def validate_subscription_weeks(weeks: int) -> None:
    if weeks < 4 or weeks % 4 != 0:
        raise InvalidInputError(
            "subscription duration must be at least 4 weeks and a multiple of 4"
        )


def request_devnet_faucet_instruction(
    program_id: Pubkey, accounts: RequestDevnetFaucetAccounts
) -> Instruction:
    return Instruction(
        program_id=program_id,
        accounts=[
            AccountMeta.writable(accounts.user, True),
            AccountMeta.writable(accounts.faucet_tracker),
            AccountMeta.writable(accounts.usdt_mint),
            AccountMeta.writable(accounts.user_usdt_ata),
            AccountMeta.readonly(accounts.usdt_treasury_pda),
            AccountMeta.readonly(accounts.token_program),
            AccountMeta.readonly(accounts.associated_token_program),
            AccountMeta.readonly(accounts.system_program),
        ],
        data=REQUEST_DEVNET_FAUCET_DISCRIMINATOR,
    )


def purchase_subscription_token_usdt_instruction(
    program_id: Pubkey, accounts: PurchaseSubscriptionTokenUsdtAccounts, txline_amount: int
) -> Instruction:
    validate_quote_amount(txline_amount)
    data = bytearray(PURCHASE_SUBSCRIPTION_TOKEN_USDT_DISCRIMINATOR)
    _put_u64(data, txline_amount)
    return Instruction(
        program_id=program_id,
        accounts=[
            AccountMeta.writable(accounts.buyer, True),
            AccountMeta.readonly(accounts.backend_admin, True),
            AccountMeta.readonly(accounts.usdt_mint),
            AccountMeta.writable(accounts.buyer_usdt_account),
            AccountMeta.writable(accounts.usdt_treasury_vault),
            AccountMeta.readonly(accounts.usdt_treasury_pda),
            AccountMeta.readonly(accounts.subscription_token_mint),
            AccountMeta.writable(accounts.token_treasury_vault),
            AccountMeta.readonly(accounts.token_treasury_pda),
            AccountMeta.writable(accounts.buyer_token_account),
            AccountMeta.readonly(accounts.token_program),
            AccountMeta.readonly(accounts.token_2022_program),
            AccountMeta.readonly(accounts.system_program),
            AccountMeta.readonly(accounts.associated_token_program),
        ],
        data=bytes(data),
    )


def create_token_2022_associated_token_account_instruction(
    payer: str | Pubkey,
    associated_token_account: str | Pubkey,
    owner: str | Pubkey,
    mint: str | Pubkey,
) -> Instruction:
    return Instruction(
        program_id=ensure_pubkey(ASSOCIATED_TOKEN_PROGRAM_ID),
        accounts=[
            AccountMeta.writable(ensure_pubkey(payer), True),
            AccountMeta.writable(ensure_pubkey(associated_token_account)),
            AccountMeta.readonly(ensure_pubkey(owner)),
            AccountMeta.readonly(ensure_pubkey(mint)),
            AccountMeta.readonly(ensure_pubkey(SYSTEM_PROGRAM_ID)),
            AccountMeta.readonly(ensure_pubkey(TOKEN_2022_PROGRAM_ID)),
        ],
        data=b"",
    )


def validate_stat_instruction(
    program_id: Pubkey,
    daily_scores_merkle_roots: Pubkey,
    validation: ScoresStatValidation,
    predicate: TraderPredicate,
    op: BinaryExpression | None,
) -> Instruction:
    stat_a = validation.primary_stat_term()
    stat_b = validation.secondary_stat_term()
    data = bytearray(VALIDATE_STAT_DISCRIMINATOR)
    _put_i64(data, validation.summary.update_stats.min_timestamp)
    _encode_scores_batch_summary(data, validation.fixture_summary_input())
    _encode_proof_vec(data, validation.sub_tree_proof)
    _encode_proof_vec(data, validation.main_tree_proof)
    _encode_trader_predicate(data, predicate)
    _encode_stat_term(data, stat_a)
    _encode_option(data, stat_b, _encode_stat_term)
    _encode_option(data, op, lambda out, value: _encode_binary_expression(out, value))
    return Instruction(
        program_id=program_id,
        accounts=[AccountMeta.readonly(daily_scores_merkle_roots)],
        data=bytes(data),
    )


def validate_stat_v2_instruction(
    program_id: Pubkey,
    daily_scores_merkle_roots: Pubkey,
    payload: StatValidationInput,
    strategy: NDimensionalStrategy,
) -> Instruction:
    strategy.validate_indices(len(payload.stats))
    data = bytearray(VALIDATE_STAT_V2_DISCRIMINATOR)
    _encode_stat_validation_input(data, payload)
    _encode_ndimensional_strategy(data, strategy)
    return Instruction(
        program_id=program_id,
        accounts=[AccountMeta.readonly(daily_scores_merkle_roots)],
        data=bytes(data),
    )


def validate_fixture_instruction(
    program_id: Pubkey, ten_daily_fixtures_roots: Pubkey, validation: FixtureValidation
) -> Instruction:
    data = bytearray(VALIDATE_FIXTURE_DISCRIMINATOR)
    _encode_fixture(data, validation.snapshot)
    _encode_fixture_batch_summary(data, validation.summary)
    _encode_proof_vec(data, validation.sub_tree_proof)
    _encode_proof_vec(data, validation.main_tree_proof)
    return Instruction(
        program_id=program_id,
        accounts=[AccountMeta.readonly(ten_daily_fixtures_roots)],
        data=bytes(data),
    )


def validate_fixture_batch_instruction(
    program_id: Pubkey,
    ten_daily_fixtures_roots: Pubkey,
    index: int,
    validation: FixtureBatchValidation,
) -> Instruction:
    data = bytearray(VALIDATE_FIXTURE_BATCH_DISCRIMINATOR)
    _put_u8(data, index)
    _encode_batch_metadata(data, validation.metadata)
    _encode_proof_vec(data, validation.proof)
    return Instruction(
        program_id=program_id,
        accounts=[AccountMeta.readonly(ten_daily_fixtures_roots)],
        data=bytes(data),
    )


def validate_odds_instruction(
    program_id: Pubkey, daily_odds_merkle_roots: Pubkey, validation: OddsValidation
) -> Instruction:
    data = bytearray(VALIDATE_ODDS_DISCRIMINATOR)
    _put_i64(data, validation.odds.ts)
    _encode_odds(data, validation.odds)
    _encode_odds_batch_summary(data, validation.summary)
    _encode_proof_vec(data, validation.sub_tree_proof)
    _encode_proof_vec(data, validation.main_tree_proof)
    return Instruction(
        program_id=program_id,
        accounts=[AccountMeta.readonly(daily_odds_merkle_roots)],
        data=bytes(data),
    )


@dataclass(frozen=True, slots=True)
class CreateIntentAccounts:
    maker: Pubkey
    order_intent: Pubkey
    intent_vault: Pubkey
    maker_token_account: Pubkey
    token_mint: Pubkey
    token_treasury_pda: Pubkey
    token_program: Pubkey
    system_program: Pubkey


@dataclass(frozen=True, slots=True)
class CreateIntentParams:
    intent_id: int
    terms_hash: bytes
    deposit_amount: int
    expiration_ts: int
    claim_period: int
    fixture_id: int


def create_intent_instruction(
    program_id: Pubkey, accounts: CreateIntentAccounts, params: CreateIntentParams
) -> Instruction:
    data = bytearray(CREATE_INTENT_DISCRIMINATOR)
    _put_u64(data, params.intent_id)
    _put_hash(data, params.terms_hash)
    _put_u64(data, params.deposit_amount)
    _put_i64(data, params.expiration_ts)
    _put_u16(data, params.claim_period)
    _put_i64(data, params.fixture_id)
    return Instruction(
        program_id,
        [
            AccountMeta.writable(accounts.maker, True),
            AccountMeta.writable(accounts.order_intent),
            AccountMeta.writable(accounts.intent_vault),
            AccountMeta.writable(accounts.maker_token_account),
            AccountMeta.readonly(accounts.token_mint),
            AccountMeta.readonly(accounts.token_treasury_pda),
            AccountMeta.readonly(accounts.token_program),
            AccountMeta.readonly(accounts.system_program),
        ],
        bytes(data),
    )


@dataclass(frozen=True, slots=True)
class CreateTradeAccounts:
    authority: Pubkey
    trader_a: Pubkey
    trader_b: Pubkey
    trader_a_token_account: Pubkey
    trader_b_token_account: Pubkey
    trade_escrow: Pubkey
    escrow_vault: Pubkey
    stake_token_mint: Pubkey
    token_treasury_pda: Pubkey
    token_program: Pubkey
    system_program: Pubkey


@dataclass(frozen=True, slots=True)
class CreateTradeParams:
    trade_id: int
    stake_a: int
    stake_b: int
    trade_terms_hash: bytes


def create_trade_instruction(
    program_id: Pubkey, accounts: CreateTradeAccounts, params: CreateTradeParams
) -> Instruction:
    data = bytearray(CREATE_TRADE_DISCRIMINATOR)
    _put_u64(data, params.trade_id)
    _put_u64(data, params.stake_a)
    _put_u64(data, params.stake_b)
    _put_hash(data, params.trade_terms_hash)
    return Instruction(
        program_id,
        [
            AccountMeta.writable(accounts.authority, True),
            AccountMeta.writable(accounts.trader_a, True),
            AccountMeta.writable(accounts.trader_b, True),
            AccountMeta.writable(accounts.trader_a_token_account),
            AccountMeta.writable(accounts.trader_b_token_account),
            AccountMeta.writable(accounts.trade_escrow),
            AccountMeta.writable(accounts.escrow_vault),
            AccountMeta.readonly(accounts.stake_token_mint),
            AccountMeta.readonly(accounts.token_treasury_pda),
            AccountMeta.readonly(accounts.token_program),
            AccountMeta.readonly(accounts.system_program),
        ],
        bytes(data),
    )


@dataclass(frozen=True, slots=True)
class ExecuteMatchAccounts:
    solver: Pubkey
    maker_intent: Pubkey
    taker_intent: Pubkey
    maker_vault: Pubkey
    taker_vault: Pubkey
    matched_trade: Pubkey
    trade_vault: Pubkey
    token_mint: Pubkey
    token_program: Pubkey
    system_program: Pubkey


@dataclass(frozen=True, slots=True)
class ExecuteMatchParams:
    trade_id: int
    maker_stake: int
    taker_stake: int


def execute_match_instruction(
    program_id: Pubkey, accounts: ExecuteMatchAccounts, params: ExecuteMatchParams
) -> Instruction:
    data = bytearray(EXECUTE_MATCH_DISCRIMINATOR)
    _put_u64(data, params.trade_id)
    _put_u64(data, params.maker_stake)
    _put_u64(data, params.taker_stake)
    return Instruction(
        program_id,
        [
            AccountMeta.writable(accounts.solver, True),
            AccountMeta.writable(accounts.maker_intent),
            AccountMeta.writable(accounts.taker_intent),
            AccountMeta.writable(accounts.maker_vault),
            AccountMeta.writable(accounts.taker_vault),
            AccountMeta.writable(accounts.matched_trade),
            AccountMeta.writable(accounts.trade_vault),
            AccountMeta.readonly(accounts.token_mint),
            AccountMeta.readonly(accounts.token_program),
            AccountMeta.readonly(accounts.system_program),
        ],
        bytes(data),
    )


@dataclass(frozen=True, slots=True)
class CloseIntentAccounts:
    maker: Pubkey
    authority: Pubkey
    order_intent: Pubkey
    intent_vault: Pubkey
    maker_token_account: Pubkey
    token_mint: Pubkey
    token_program: Pubkey
    token_treasury_pda: Pubkey


def close_intent_instruction(program_id: Pubkey, accounts: CloseIntentAccounts) -> Instruction:
    return Instruction(
        program_id,
        [
            AccountMeta.writable(accounts.maker),
            AccountMeta.writable(accounts.authority, True),
            AccountMeta.writable(accounts.order_intent),
            AccountMeta.writable(accounts.intent_vault),
            AccountMeta.writable(accounts.maker_token_account),
            AccountMeta.readonly(accounts.token_mint),
            AccountMeta.readonly(accounts.token_program),
            AccountMeta.readonly(accounts.token_treasury_pda),
        ],
        CLOSE_INTENT_DISCRIMINATOR,
    )


@dataclass(frozen=True, slots=True)
class SettleTradeAccounts:
    winner: Pubkey
    daily_scores_merkle_roots: Pubkey
    trade_escrow: Pubkey
    escrow_vault: Pubkey
    winner_token_account: Pubkey
    token_mint: Pubkey
    token_treasury_pda: Pubkey
    token_program: Pubkey
    system_program: Pubkey


@dataclass(frozen=True, slots=True)
class SettleTradeParams:
    trade_id: int
    ts: int
    fixture_summary: FixtureSummaryInput
    fixture_proof: list[ProofNode]
    main_tree_proof: list[ProofNode]
    predicate: TraderPredicate
    stat_a: StatTermInput
    stat_b: StatTermInput | None
    op: BinaryExpression | None


def settle_trade_instruction(
    program_id: Pubkey, accounts: SettleTradeAccounts, params: SettleTradeParams
) -> Instruction:
    data = bytearray(SETTLE_TRADE_DISCRIMINATOR)
    _put_u64(data, params.trade_id)
    _put_i64(data, params.ts)
    _encode_scores_batch_summary(data, params.fixture_summary)
    _encode_proof_vec(data, params.fixture_proof)
    _encode_proof_vec(data, params.main_tree_proof)
    _encode_trader_predicate(data, params.predicate)
    _encode_stat_term(data, params.stat_a)
    _encode_option(data, params.stat_b, _encode_stat_term)
    _encode_option(data, params.op, lambda out, value: _encode_binary_expression(out, value))
    return Instruction(
        program_id,
        [
            AccountMeta.writable(accounts.winner, True),
            AccountMeta.readonly(accounts.daily_scores_merkle_roots),
            AccountMeta.writable(accounts.trade_escrow),
            AccountMeta.writable(accounts.escrow_vault),
            AccountMeta.writable(accounts.winner_token_account),
            AccountMeta.readonly(accounts.token_mint),
            AccountMeta.readonly(accounts.token_treasury_pda),
            AccountMeta.readonly(accounts.token_program),
            AccountMeta.readonly(accounts.system_program),
        ],
        bytes(data),
    )


@dataclass(frozen=True, slots=True)
class MarketIntentParams:
    fixture_id: int
    period: int
    stat_a_key: int
    stat_b_key: int | None
    predicate: TraderPredicate
    op: BinaryExpression | None
    negation: bool


@dataclass(frozen=True, slots=True)
class SettleMatchedTradeAccounts:
    winner: Pubkey
    daily_scores_merkle_roots: Pubkey
    matched_trade: Pubkey
    trade_vault: Pubkey
    winner_token_account: Pubkey
    token_mint: Pubkey
    token_treasury_pda: Pubkey
    token_program: Pubkey
    system_program: Pubkey


@dataclass(frozen=True, slots=True)
class SettleMatchedTradeParams:
    trade_id: int
    ts: int
    fixture_summary: FixtureSummaryInput
    fixture_proof: list[ProofNode]
    main_tree_proof: list[ProofNode]
    stat_a: StatTermInput
    stat_b: StatTermInput | None
    terms: MarketIntentParams


def settle_matched_trade_instruction(
    program_id: Pubkey, accounts: SettleMatchedTradeAccounts, params: SettleMatchedTradeParams
) -> Instruction:
    data = bytearray(SETTLE_MATCHED_TRADE_DISCRIMINATOR)
    _put_u64(data, params.trade_id)
    _put_i64(data, params.ts)
    _encode_scores_batch_summary(data, params.fixture_summary)
    _encode_proof_vec(data, params.fixture_proof)
    _encode_proof_vec(data, params.main_tree_proof)
    _encode_stat_term(data, params.stat_a)
    _encode_option(data, params.stat_b, _encode_stat_term)
    _encode_market_intent_params(data, params.terms)
    return Instruction(
        program_id,
        [
            AccountMeta.writable(accounts.winner, True),
            AccountMeta.readonly(accounts.daily_scores_merkle_roots),
            AccountMeta.writable(accounts.matched_trade),
            AccountMeta.writable(accounts.trade_vault),
            AccountMeta.writable(accounts.winner_token_account),
            AccountMeta.readonly(accounts.token_mint),
            AccountMeta.readonly(accounts.token_treasury_pda),
            AccountMeta.readonly(accounts.token_program),
            AccountMeta.readonly(accounts.system_program),
        ],
        bytes(data),
    )


@dataclass(frozen=True, slots=True)
class ClaimViaResolutionAccounts:
    winner: Pubkey
    daily_resolution_roots: Pubkey
    matched_trade: Pubkey
    trade_vault: Pubkey
    winner_token_account: Pubkey
    token_program: Pubkey


@dataclass(frozen=True, slots=True)
class ClaimViaResolutionParams:
    epoch_day: int
    interval_index: int
    merkle_proof: list[ProofNode]


def claim_via_resolution_instruction(
    program_id: Pubkey, accounts: ClaimViaResolutionAccounts, params: ClaimViaResolutionParams
) -> Instruction:
    data = bytearray(CLAIM_VIA_RESOLUTION_DISCRIMINATOR)
    _put_u16(data, params.epoch_day)
    _put_u16(data, params.interval_index)
    _encode_proof_vec(data, params.merkle_proof)
    return Instruction(
        program_id,
        [
            AccountMeta.writable(accounts.winner, True),
            AccountMeta.readonly(accounts.daily_resolution_roots),
            AccountMeta.writable(accounts.matched_trade),
            AccountMeta.writable(accounts.trade_vault),
            AccountMeta.writable(accounts.winner_token_account),
            AccountMeta.readonly(accounts.token_program),
        ],
        bytes(data),
    )


@dataclass(frozen=True, slots=True)
class ClaimBatchLegacyAccounts:
    payer: Pubkey
    daily_resolution_roots: Pubkey
    token_mint: Pubkey
    token_program: Pubkey
    system_program: Pubkey


@dataclass(frozen=True, slots=True)
class ClaimBatchLegacyParams:
    epoch_day: int
    interval_index: int
    terms_hash: bytes
    winner_is_maker: bool
    seq: int
    merkle_proof: list[ProofNode]


def claim_batch_legacy_instruction(
    program_id: Pubkey, accounts: ClaimBatchLegacyAccounts, params: ClaimBatchLegacyParams
) -> Instruction:
    data = bytearray(CLAIM_BATCH_LEGACY_DISCRIMINATOR)
    _put_u16(data, params.epoch_day)
    _put_u16(data, params.interval_index)
    _put_hash(data, params.terms_hash)
    _put_bool(data, params.winner_is_maker)
    _put_u32(data, params.seq)
    _encode_proof_vec(data, params.merkle_proof)
    return Instruction(
        program_id,
        [
            AccountMeta.writable(accounts.payer, True),
            AccountMeta.readonly(accounts.daily_resolution_roots),
            AccountMeta.readonly(accounts.token_mint),
            AccountMeta.readonly(accounts.token_program),
            AccountMeta.readonly(accounts.system_program),
        ],
        bytes(data),
    )


@dataclass(frozen=True, slots=True)
class RefundBatchAccounts:
    payer: Pubkey
    token_mint: Pubkey
    token_program: Pubkey
    system_program: Pubkey


def refund_batch_instruction(program_id: Pubkey, accounts: RefundBatchAccounts) -> Instruction:
    return Instruction(
        program_id,
        [
            AccountMeta.writable(accounts.payer, True),
            AccountMeta.readonly(accounts.token_mint),
            AccountMeta.readonly(accounts.token_program),
            AccountMeta.readonly(accounts.system_program),
        ],
        REFUND_BATCH_DISCRIMINATOR,
    )


@dataclass(frozen=True, slots=True)
class AuditTradeResultAccounts:
    payer: Pubkey
    daily_scores_merkle_roots: Pubkey


@dataclass(frozen=True, slots=True)
class AuditTradeResultParams:
    terms: MarketIntentParams
    fixture_summary: FixtureSummaryInput
    main_tree_proof: list[ProofNode]
    fixture_proof: list[ProofNode]
    stat_a: StatTermInput
    stat_b: StatTermInput | None
    ts: int


def audit_trade_result_instruction(
    program_id: Pubkey, accounts: AuditTradeResultAccounts, params: AuditTradeResultParams
) -> Instruction:
    data = bytearray(AUDIT_TRADE_RESULT_DISCRIMINATOR)
    _encode_market_intent_params(data, params.terms)
    _encode_scores_batch_summary(data, params.fixture_summary)
    _encode_proof_vec(data, params.main_tree_proof)
    _encode_proof_vec(data, params.fixture_proof)
    _encode_stat_term(data, params.stat_a)
    _encode_option(data, params.stat_b, _encode_stat_term)
    _put_i64(data, params.ts)
    return Instruction(
        program_id,
        [
            AccountMeta.writable(accounts.payer, True),
            AccountMeta.readonly(accounts.daily_scores_merkle_roots),
        ],
        bytes(data),
    )


def _encode_stat_validation_input(out: bytearray, input_data: StatValidationInput) -> None:
    _put_i64(out, input_data.ts)
    _encode_scores_batch_summary(out, input_data.fixture_summary)
    _encode_proof_vec(out, input_data.fixture_proof)
    _encode_proof_vec(out, input_data.main_tree_proof)
    _put_hash(out, input_data.event_stat_root)
    _put_vec(out, input_data.stats, _encode_stat_leaf)


def _encode_stat_leaf(out: bytearray, leaf: StatLeafInput) -> None:
    _encode_score_stat(out, leaf.stat)
    _encode_proof_vec(out, leaf.stat_proof)


def _encode_ndimensional_strategy(out: bytearray, strategy: NDimensionalStrategy) -> None:
    _put_vec(out, strategy.geometric_targets, _encode_geometric_target)
    _encode_option(out, strategy.distance_predicate, _encode_trader_predicate)
    _put_vec(out, strategy.discrete_predicates, _encode_stat_predicate)


def _encode_geometric_target(out: bytearray, target: GeometricTarget) -> None:
    _put_u8(out, target.stat_index)
    _put_i32(out, target.prediction)


def _encode_stat_predicate(out: bytearray, predicate: SinglePredicate | BinaryPredicate) -> None:
    if isinstance(predicate, SinglePredicate):
        _put_u8(out, 0)
        _put_u8(out, predicate.index)
        _encode_trader_predicate(out, predicate.predicate)
    else:
        _put_u8(out, 1)
        _put_u8(out, predicate.index_a)
        _put_u8(out, predicate.index_b)
        _encode_binary_expression(out, predicate.op)
        _encode_trader_predicate(out, predicate.predicate)


def _encode_fixture(out: bytearray, fixture: Fixture) -> None:
    _put_i64(out, fixture.ts)
    _put_i64(out, fixture.start_time)
    _put_string(out, fixture.competition)
    _put_i32(out, fixture.competition_id)
    _put_i32(out, fixture.fixture_group_id)
    _put_i32(out, fixture.participant1_id)
    _put_string(out, fixture.participant1)
    _put_i32(out, fixture.participant2_id)
    _put_string(out, fixture.participant2)
    _put_i64(out, fixture.fixture_id)
    _put_bool(out, fixture.participant1_is_home)


def _encode_fixture_batch_summary(out: bytearray, summary: FixtureBatchSummary) -> None:
    _put_i64(out, summary.fixture_id)
    _put_i32(out, summary.competition_id)
    _put_string(out, summary.competition)
    _encode_update_stats_u32(
        out,
        summary.update_stats.update_count,
        summary.update_stats.min_timestamp,
        summary.update_stats.max_timestamp,
    )
    _put_hash(out, summary.update_sub_tree_root.as_bytes())


def _encode_batch_metadata(out: bytearray, metadata: BatchMetadata) -> None:
    _put_i32(out, metadata.total_update_count)
    _put_i32(out, metadata.num_unique_fixtures)
    _put_i64(out, metadata.overall_batch_start_ts)
    _put_i64(out, metadata.overall_batch_end_ts)


def _encode_odds(out: bytearray, odds: OddsPayload) -> None:
    _put_i64(out, odds.fixture_id)
    _put_string(out, odds.message_id)
    _put_i64(out, odds.ts)
    _put_string(out, odds.bookmaker)
    _put_i32(out, odds.bookmaker_id)
    _put_string(out, odds.super_odds_type)
    _encode_string_option(out, odds.game_state)
    _put_bool(out, odds.in_running)
    _encode_string_option(out, odds.market_parameters)
    _encode_string_option(out, odds.market_period)
    _put_vec(out, odds.price_names, lambda buf, value: _put_string(buf, value))
    _put_vec(out, odds.prices, lambda buf, value: _put_i32(buf, value))


def _encode_odds_batch_summary(out: bytearray, summary: OddsBatchSummary) -> None:
    _put_i64(out, summary.fixture_id)
    _encode_update_stats_u32(
        out,
        summary.update_stats.update_count,
        summary.update_stats.min_timestamp,
        summary.update_stats.max_timestamp,
    )
    _put_hash(out, summary.odds_sub_tree_root.as_bytes())


def _encode_scores_batch_summary(out: bytearray, summary: FixtureSummaryInput) -> None:
    _put_i64(out, summary.fixture_id)
    _put_i32(out, summary.update_count)
    _put_i64(out, summary.min_timestamp)
    _put_i64(out, summary.max_timestamp)
    _put_hash(out, summary.events_sub_tree_root)


def _encode_stat_term(out: bytearray, term: StatTermInput) -> None:
    _encode_score_stat(out, term.stat_to_prove)
    _put_hash(out, term.event_stat_root)
    _encode_proof_vec(out, term.stat_proof)


def _encode_score_stat(out: bytearray, stat: ScoreStat) -> None:
    _put_u32(out, stat.key)
    _put_i32(out, stat.value)
    _put_i32(out, stat.period)


def _encode_proof_vec(out: bytearray, proof: list[ProofNode]) -> None:
    _put_vec(out, proof, _encode_proof_node)


def _encode_proof_node(out: bytearray, node: ProofNode) -> None:
    _put_hash(out, node.hash.as_bytes())
    _put_bool(out, node.is_right_sibling)


def _encode_trader_predicate(out: bytearray, predicate: TraderPredicate) -> None:
    _put_i32(out, predicate.threshold)
    _encode_comparison(out, predicate.comparison)


def _encode_comparison(out: bytearray, comparison: Comparison) -> None:
    mapping = {
        Comparison.GREATER_THAN: 0,
        Comparison.LESS_THAN: 1,
        Comparison.EQUAL_TO: 2,
    }
    _put_u8(out, mapping[comparison])


def _encode_binary_expression(out: bytearray, op: BinaryExpression) -> None:
    _put_u8(out, 0 if op == BinaryExpression.ADD else 1)


def _encode_market_intent_params(out: bytearray, terms: MarketIntentParams) -> None:
    _put_i64(out, terms.fixture_id)
    _put_u16(out, terms.period)
    _put_u32(out, terms.stat_a_key)
    _encode_option(out, terms.stat_b_key, lambda buf, value: _put_u32(buf, value))
    _encode_trader_predicate(out, terms.predicate)
    _encode_option(out, terms.op, lambda buf, value: _encode_binary_expression(buf, value))
    _put_bool(out, terms.negation)


def _encode_option(
    out: bytearray, value: _T | None, encoder: Callable[[bytearray, _T], None]
) -> None:
    if value is None:
        _put_u8(out, 0)
    else:
        _put_u8(out, 1)
        encoder(out, value)


def _encode_string_option(out: bytearray, value: str | None) -> None:
    if value is None:
        _put_u8(out, 0)
    else:
        _put_u8(out, 1)
        _put_string(out, value)


def _encode_update_stats_u32(
    out: bytearray, update_count: int, min_timestamp: int, max_timestamp: int
) -> None:
    if update_count < 0:
        raise ValidationError("update_count must be nonnegative to match the Devnet IDL u32 field")
    _put_u32(out, update_count)
    _put_i64(out, min_timestamp)
    _put_i64(out, max_timestamp)


def _put_vec(
    out: bytearray, values: Sequence[_T], encoder: Callable[[bytearray, _T], None]
) -> None:
    _put_u32(out, len(values))
    for value in values:
        encoder(out, value)


def _put_string(out: bytearray, value: str) -> None:
    data = value.encode("utf-8")
    _put_u32(out, len(data))
    out.extend(data)


def _put_hash(out: bytearray, value: bytes) -> None:
    if len(value) != 32:
        raise ValidationError(f"expected 32-byte hash, got {len(value)}")
    out.extend(value)


def _put_bool(out: bytearray, value: bool) -> None:
    _put_u8(out, 1 if value else 0)


def _put_u8(out: bytearray, value: int) -> None:
    out.extend(struct.pack("<B", value))


def _put_u16(out: bytearray, value: int) -> None:
    out.extend(struct.pack("<H", value))


def _put_u32(out: bytearray, value: int) -> None:
    out.extend(struct.pack("<I", value))


def _put_u64(out: bytearray, value: int) -> None:
    out.extend(struct.pack("<Q", value))


def _put_i32(out: bytearray, value: int) -> None:
    out.extend(struct.pack("<i", value))


def _put_i64(out: bytearray, value: int) -> None:
    out.extend(struct.pack("<q", value))
