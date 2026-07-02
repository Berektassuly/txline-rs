//! Minimal Anchor/Borsh encoding helpers shared by Devnet instruction builders.

use crate::validation::legacy::{FixtureSummaryInput, ScoreStat, StatTermInput};
use crate::validation::proof::ProofNode;
use crate::validation::strategy::{BinaryExpression, Comparison, TraderPredicate};
use crate::{Result, TxlineError};

pub(crate) fn encode_stat_term(out: &mut Vec<u8>, term: &StatTermInput) -> Result<()> {
    encode_score_stat(out, &term.stat_to_prove);
    out.extend_from_slice(&term.event_stat_root);
    encode_proof_vec(out, &term.stat_proof)
}

pub(crate) fn encode_scores_batch_summary(out: &mut Vec<u8>, summary: &FixtureSummaryInput) {
    put_i64(out, summary.fixture_id);
    put_i32(out, summary.update_count);
    put_i64(out, summary.min_timestamp);
    put_i64(out, summary.max_timestamp);
    out.extend_from_slice(&summary.events_sub_tree_root);
}

pub(crate) fn encode_score_stat(out: &mut Vec<u8>, stat: &ScoreStat) {
    put_u32(out, stat.key);
    put_i32(out, stat.value);
    put_i32(out, stat.period);
}

pub(crate) fn encode_proof_vec(out: &mut Vec<u8>, proof: &[ProofNode]) -> Result<()> {
    put_vec(out, proof, |out, node| {
        out.extend_from_slice(node.hash.as_bytes());
        put_bool(out, node.is_right_sibling);
        Ok(())
    })
}

pub(crate) fn encode_trader_predicate(out: &mut Vec<u8>, predicate: &TraderPredicate) {
    put_i32(out, predicate.threshold);
    encode_comparison(out, &predicate.comparison);
}

pub(crate) fn encode_comparison(out: &mut Vec<u8>, comparison: &Comparison) {
    let variant = match comparison {
        Comparison::GreaterThan {} => 0,
        Comparison::LessThan {} => 1,
        Comparison::EqualTo {} => 2,
    };
    put_u8(out, variant);
}

pub(crate) fn encode_binary_expression(out: &mut Vec<u8>, op: &BinaryExpression) {
    let variant = match op {
        BinaryExpression::Add {} => 0,
        BinaryExpression::Subtract {} => 1,
    };
    put_u8(out, variant);
}

pub(crate) fn encode_option<T>(
    out: &mut Vec<u8>,
    value: Option<&T>,
    encode: impl Fn(&mut Vec<u8>, &T) -> Result<()>,
) -> Result<()> {
    match value {
        Some(value) => {
            put_u8(out, 1);
            encode(out, value)
        }
        None => {
            put_u8(out, 0);
            Ok(())
        }
    }
}

pub(crate) fn encode_string_option(out: &mut Vec<u8>, value: Option<&str>) -> Result<()> {
    match value {
        Some(value) => {
            put_u8(out, 1);
            put_string(out, value)
        }
        None => {
            put_u8(out, 0);
            Ok(())
        }
    }
}

pub(crate) fn put_vec<T>(
    out: &mut Vec<u8>,
    values: &[T],
    encode: impl Fn(&mut Vec<u8>, &T) -> Result<()>,
) -> Result<()> {
    put_u32(out, vec_len(values.len())?);
    for value in values {
        encode(out, value)?;
    }
    Ok(())
}

pub(crate) fn put_string(out: &mut Vec<u8>, value: &str) -> Result<()> {
    put_u32(out, vec_len(value.len())?);
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn vec_len(len: usize) -> Result<u32> {
    u32::try_from(len).map_err(|_| TxlineError::validation("Anchor vector length exceeds u32"))
}

pub(crate) fn put_bool(out: &mut Vec<u8>, value: bool) {
    put_u8(out, u8::from(value));
}

pub(crate) fn put_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

pub(crate) fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}
