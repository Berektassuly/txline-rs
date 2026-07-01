//! Legacy `/api/scores/stat-validation` response types.

use serde::{Deserialize, Serialize};

use super::proof::{Hash32, ProofNode};
use crate::http::models::UpdateStats;
use crate::{Result, TxlineError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreStat {
    pub key: u32,
    pub value: i32,
    pub period: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoresBatchSummary {
    pub fixture_id: i64,
    pub update_stats: UpdateStats,
    pub event_stats_sub_tree_root: Hash32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoresStatValidation {
    pub ts: i64,
    pub stat_to_prove: ScoreStat,
    pub event_stat_root: Hash32,
    pub summary: ScoresBatchSummary,
    #[serde(default)]
    pub stat_proof: Vec<ProofNode>,
    #[serde(default)]
    pub sub_tree_proof: Vec<ProofNode>,
    #[serde(default)]
    pub main_tree_proof: Vec<ProofNode>,
    #[serde(default)]
    pub stat_to_prove2: Option<ScoreStat>,
    #[serde(default)]
    pub stat_proof2: Option<Vec<ProofNode>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureSummaryInput {
    pub fixture_id: i64,
    pub update_count: i32,
    pub min_timestamp: i64,
    pub max_timestamp: i64,
    pub events_sub_tree_root: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatTermInput {
    pub stat_to_prove: ScoreStat,
    pub event_stat_root: [u8; 32],
    pub stat_proof: Vec<ProofNode>,
}

impl ScoresStatValidation {
    pub fn fixture_summary_input(&self) -> FixtureSummaryInput {
        FixtureSummaryInput {
            fixture_id: self.summary.fixture_id,
            update_count: self.summary.update_stats.update_count,
            min_timestamp: self.summary.update_stats.min_timestamp,
            max_timestamp: self.summary.update_stats.max_timestamp,
            events_sub_tree_root: self.summary.event_stats_sub_tree_root.to_bytes(),
        }
    }

    pub fn primary_stat_term(&self) -> StatTermInput {
        StatTermInput {
            stat_to_prove: self.stat_to_prove.clone(),
            event_stat_root: self.event_stat_root.to_bytes(),
            stat_proof: self.stat_proof.clone(),
        }
    }

    pub fn secondary_stat_term(&self) -> Result<Option<StatTermInput>> {
        match (&self.stat_to_prove2, &self.stat_proof2) {
            (Some(stat_to_prove), Some(stat_proof)) => Ok(Some(StatTermInput {
                stat_to_prove: stat_to_prove.clone(),
                event_stat_root: self.event_stat_root.to_bytes(),
                stat_proof: stat_proof.clone(),
            })),
            (None, None) => Ok(None),
            _ => Err(TxlineError::validation(
                "legacy response contains only one of statToProve2/statProof2",
            )),
        }
    }

    pub fn epoch_day(&self) -> Result<u16> {
        timestamp_ms_to_epoch_day(self.summary.update_stats.min_timestamp)
    }
}

pub(crate) fn ensure_positive_seq(seq: i32) -> Result<()> {
    if seq <= 0 {
        return Err(TxlineError::invalid_input(
            "score stat validation seq must be greater than zero and must come from a real score record",
        ));
    }
    Ok(())
}

pub fn timestamp_ms_to_epoch_day(timestamp_ms: i64) -> Result<u16> {
    if timestamp_ms < 0 {
        return Err(TxlineError::validation(
            "validation timestamp must not be negative",
        ));
    }
    let epoch_day = timestamp_ms / 86_400_000;
    u16::try_from(epoch_day)
        .map_err(|_| TxlineError::validation("epoch day does not fit into u16 PDA seed"))
}
