//! Conservative purchase-quote safety primitives.
//!
//! This pass does not include a full decoded Solana transaction audit. The
//! helpers here intentionally stop at checks the SDK can perform without Anchor
//! program bindings: amount bounds, base64 transaction decoding, and financial
//! consistency. Callers should still inspect fee payer, signer set, invoked
//! programs, account metas, and decoded oracle instruction before signing paid
//! purchase transactions.

use super::pda::{
    ASSOCIATED_TOKEN_PROGRAM_ID, COMPUTE_BUDGET_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
};

pub const LEGACY_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

pub fn allowed_purchase_programs(txline_program_id: &str) -> [&str; 6] {
    [
        txline_program_id,
        COMPUTE_BUDGET_PROGRAM_ID,
        SYSTEM_PROGRAM_ID,
        LEGACY_TOKEN_PROGRAM_ID,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
    ]
}
