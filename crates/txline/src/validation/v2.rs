//! V2 score-validation response and payload helpers.

use serde::{Deserialize, Serialize};

use super::legacy::{
    FixtureSummaryInput, ScoreStat, ScoresBatchSummary, timestamp_ms_to_epoch_day,
};
use super::proof::{Hash32, ProofNode};
use crate::{Result, TxlineError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoresStatValidationV2Response {
    pub ts: i64,
    #[serde(default)]
    pub stats_to_prove: Vec<ScoreStat>,
    pub event_stat_root: Hash32,
    pub summary: ScoresBatchSummary,
    #[serde(default)]
    pub stat_proofs: Vec<Vec<ProofNode>>,
    #[serde(default)]
    pub sub_tree_proof: Vec<ProofNode>,
    #[serde(default)]
    pub main_tree_proof: Vec<ProofNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScoresStatValidationV2 {
    requested_stat_keys: Vec<u32>,
    response: ScoresStatValidationV2Response,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatLeafInput {
    pub stat: ScoreStat,
    pub stat_proof: Vec<ProofNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatValidationInput {
    pub ts: i64,
    pub fixture_summary: FixtureSummaryInput,
    pub fixture_proof: Vec<ProofNode>,
    pub main_tree_proof: Vec<ProofNode>,
    pub event_stat_root: [u8; 32],
    pub stats: Vec<StatLeafInput>,
}

impl ScoresStatValidationV2 {
    pub fn from_response(
        requested_stat_keys: Vec<u32>,
        response: ScoresStatValidationV2Response,
    ) -> Result<Self> {
        if requested_stat_keys.is_empty() {
            return Err(TxlineError::invalid_input(
                "V2 stat validation requires at least one stat key",
            ));
        }
        if response.stats_to_prove.len() != requested_stat_keys.len() {
            return Err(TxlineError::validation(format!(
                "statsToProve length {} does not match requested statKeys length {}",
                response.stats_to_prove.len(),
                requested_stat_keys.len()
            )));
        }
        for (idx, (stat, requested_key)) in response
            .stats_to_prove
            .iter()
            .zip(requested_stat_keys.iter())
            .enumerate()
        {
            if stat.key != *requested_key {
                return Err(TxlineError::validation(format!(
                    "statsToProve[{idx}].key {} does not match requested statKeys[{idx}] {}",
                    stat.key, requested_key
                )));
            }
        }
        if response.stat_proofs.len() != response.stats_to_prove.len() {
            return Err(TxlineError::validation(format!(
                "statProofs length {} does not match statsToProve length {}",
                response.stat_proofs.len(),
                response.stats_to_prove.len()
            )));
        }
        Ok(Self {
            requested_stat_keys,
            response,
        })
    }

    pub fn requested_stat_keys(&self) -> &[u32] {
        &self.requested_stat_keys
    }

    pub fn stats_to_prove(&self) -> &[ScoreStat] {
        &self.response.stats_to_prove
    }

    pub fn stat_proofs(&self) -> &[Vec<ProofNode>] {
        &self.response.stat_proofs
    }

    pub fn response(&self) -> &ScoresStatValidationV2Response {
        &self.response
    }

    /// Timestamp used by `validateStatV2` and the `daily_scores_roots` PDA.
    pub fn target_ts(&self) -> i64 {
        self.response.summary.update_stats.min_timestamp
    }

    pub fn epoch_day(&self) -> Result<u16> {
        timestamp_ms_to_epoch_day(self.target_ts())
    }

    pub fn to_validation_input(&self) -> StatValidationInput {
        let stats = self
            .response
            .stats_to_prove
            .iter()
            .cloned()
            .zip(self.response.stat_proofs.iter().cloned())
            .map(|(stat, stat_proof)| StatLeafInput { stat, stat_proof })
            .collect();

        StatValidationInput {
            ts: self.target_ts(),
            fixture_summary: FixtureSummaryInput {
                fixture_id: self.response.summary.fixture_id,
                update_count: self.response.summary.update_stats.update_count,
                min_timestamp: self.response.summary.update_stats.min_timestamp,
                max_timestamp: self.response.summary.update_stats.max_timestamp,
                events_sub_tree_root: self.response.summary.event_stats_sub_tree_root.to_bytes(),
            },
            fixture_proof: self.response.sub_tree_proof.clone(),
            main_tree_proof: self.response.main_tree_proof.clone(),
            event_stat_root: self.response.event_stat_root.to_bytes(),
            stats,
        }
    }

    pub fn leading_subset(&self, len: usize) -> Result<StatValidationInput> {
        if len == 0 || len > self.response.stats_to_prove.len() {
            return Err(TxlineError::validation(
                "V2 payload subset length must be within the proved stat count",
            ));
        }
        let mut input = self.to_validation_input();
        input.stats.truncate(len);
        Ok(input)
    }
}
