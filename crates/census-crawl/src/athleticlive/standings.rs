use serde::Deserialize;
use serde_json::Value;

use crate::hytek::{parse_field_mark, parse_time};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{EventKind, Mark};

use super::docs::value_u64;

const FINISH_SPLIT: &str = "split_final";

#[derive(Debug, Clone, Deserialize, Default)]
pub struct StandingRow {
    #[serde(default, rename = "n")]
    pub name: Option<String>,
    #[serde(default, rename = "g")]
    pub gender: Option<String>,
    #[serde(default, rename = "y")]
    pub grade: Option<Value>,
    #[serde(default, rename = "tn")]
    pub team_name: Option<String>,
    #[serde(default, rename = "ti")]
    pub team_key: Option<String>,
    #[serde(default, rename = "p")]
    pub place: Option<Value>,
    #[serde(default, rename = "m")]
    pub mark: Option<String>,
    #[serde(default, rename = "rtm")]
    pub raw_time: Option<String>,
    #[serde(default, rename = "ani")]
    pub an_athlete_id: Option<Value>,
    #[serde(default, rename = "anli")]
    pub an_legacy_id: Option<Value>,
    #[serde(default, rename = "sp")]
    pub splits: std::collections::BTreeMap<String, Value>,
}

impl StandingRow {
    pub fn name(&self) -> Option<&str> {
        self.name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
    }

    pub fn school_name(&self) -> Option<&str> {
        self.team_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
    }

    pub fn an_athlete_id(&self) -> Option<u64> {
        self.an_athlete_id.as_ref().and_then(value_u64)
    }

    pub fn has_legacy_id(&self) -> bool {
        self.an_legacy_id.as_ref().and_then(value_u64).is_some()
    }

    pub fn has_team_key(&self) -> bool {
        self.team_key
            .as_deref()
            .is_some_and(|key| !key.trim().is_empty())
    }

    pub fn place(&self) -> Option<u16> {
        let place = self.place.as_ref().and_then(value_u64)?;
        u16::try_from(place).ok().filter(|place| *place > 0)
    }

    pub fn split_count(&self) -> usize {
        self.splits
            .keys()
            .filter(|key| key.as_str() != FINISH_SPLIT)
            .count()
    }

    pub fn canonical_mark(&self, kind: &EventKind) -> Option<Mark> {
        if let Some(token) = [self.mark.as_deref(), self.raw_time.as_deref()]
            .into_iter()
            .flatten()
            .find_map(crate::result_status::invalid_token)
        {
            return Some(Mark::Raw(token.into()));
        }
        let (first, second) = if kind.is_field() {
            (self.mark.as_deref(), self.raw_time.as_deref())
        } else {
            (self.raw_time.as_deref(), self.mark.as_deref())
        };
        let token = first
            .into_iter()
            .chain(second)
            .map(str::trim)
            .find(|token| !token.is_empty())?;
        if kind.is_field() {
            return parse_field_mark(token);
        }
        parse_time(token).map(Mark::TimeSeconds)
    }

    pub(crate) fn mark_contradiction(&self, kind: &EventKind) -> Option<&str> {
        let token = [self.mark.as_deref(), self.raw_time.as_deref()]
            .into_iter()
            .flatten()
            .find_map(crate::result_status::invalid_token)?;
        let numeric = [self.mark.as_deref(), self.raw_time.as_deref()]
            .into_iter()
            .flatten()
            .any(|raw| {
                if kind.is_field() {
                    parse_field_mark(raw).is_some()
                } else {
                    parse_time(raw).is_some()
                }
            });
        numeric.then_some(token)
    }
}

pub fn parse_standings(url: &str, body: &str) -> CrawlResult<Vec<(String, StandingRow)>> {
    let rows: std::collections::BTreeMap<String, StandingRow> = serde_json::from_str(body)
        .map_err(|source| CrawlError::Decode {
            url: url.to_string(),
            source,
        })?;
    Ok(rows.into_iter().collect())
}
