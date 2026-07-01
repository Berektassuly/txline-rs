//! Scores endpoints and stat-validation request flows.

use crate::TxlineClient;
use crate::http::fixtures::{validate_hour, validate_interval};
use crate::http::models::Scores;
use crate::validation::legacy::{ScoresStatValidation, ensure_positive_seq};
use crate::validation::v2::{ScoresStatValidationV2, ScoresStatValidationV2Response};
use crate::{Result, TxlineError};

#[derive(Debug, Clone, Copy)]
pub struct ScoresClient<'a> {
    client: &'a TxlineClient,
}

impl<'a> ScoresClient<'a> {
    pub(crate) fn new(client: &'a TxlineClient) -> Self {
        Self { client }
    }

    pub async fn snapshot(&self, fixture_id: i64, as_of: Option<i64>) -> Result<Vec<Scores>> {
        let mut query = Vec::new();
        if let Some(as_of) = as_of {
            query.push(("asOf", as_of.to_string()));
        }
        self.client
            .get_json(&format!("/scores/snapshot/{fixture_id}"), query, true)
            .await
    }

    pub async fn live_updates_by_fixture(&self, fixture_id: i64) -> Result<Vec<Scores>> {
        self.client
            .get_json(&format!("/scores/updates/{fixture_id}"), Vec::new(), true)
            .await
    }

    pub async fn historical_updates(
        &self,
        epoch_day: u32,
        hour_of_day: u8,
        interval: u8,
        fixture_id: Option<i64>,
    ) -> Result<Vec<Scores>> {
        validate_hour(hour_of_day)?;
        validate_interval(interval)?;
        let mut query = Vec::new();
        if let Some(fixture_id) = fixture_id {
            query.push(("fixtureId", fixture_id.to_string()));
        }
        self.client
            .get_json(
                &format!("/scores/updates/{epoch_day}/{hour_of_day}/{interval}"),
                query,
                true,
            )
            .await
    }

    pub async fn historical_by_fixture(&self, fixture_id: i64) -> Result<Vec<Scores>> {
        self.client
            .get_json(
                &format!("/scores/historical/{fixture_id}"),
                Vec::new(),
                true,
            )
            .await
    }

    pub async fn stat_validation_legacy(
        &self,
        fixture_id: i64,
        seq: i32,
        stat_key: u32,
        stat_key2: Option<u32>,
    ) -> Result<ScoresStatValidation> {
        ensure_positive_seq(seq)?;
        let mut query = vec![
            ("fixtureId", fixture_id.to_string()),
            ("seq", seq.to_string()),
            ("statKey", stat_key.to_string()),
        ];
        if let Some(stat_key2) = stat_key2 {
            query.push(("statKey2", stat_key2.to_string()));
        }
        self.client
            .get_json("/scores/stat-validation", query, true)
            .await
    }

    pub async fn stat_validation_v2(
        &self,
        fixture_id: i64,
        seq: i32,
        stat_keys: impl IntoIterator<Item = u32>,
    ) -> Result<ScoresStatValidationV2> {
        ensure_positive_seq(seq)?;
        let stat_keys = stat_keys.into_iter().collect::<Vec<_>>();
        if stat_keys.is_empty() {
            return Err(TxlineError::invalid_input(
                "V2 stat validation requires at least one stat key",
            ));
        }
        let stat_keys_csv = stat_keys
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let response = self
            .client
            .get_json::<ScoresStatValidationV2Response>(
                "/scores/stat-validation",
                vec![
                    ("fixtureId", fixture_id.to_string()),
                    ("seq", seq.to_string()),
                    ("statKeys", stat_keys_csv),
                ],
                true,
            )
            .await?;
        ScoresStatValidationV2::from_response(stat_keys, response)
    }
}
