use super::invalid;
use census_crawl::milesplit::SourceRowLocator;
use census_domain::model::{Evidence, EvidenceMethod};
use census_store::{StoreError, StoreResult};
use chrono::{DateTime, Utc};
use serde::Deserialize;

const ROLE: &str = "roster_positive_row_metadata_v1";

#[derive(Deserialize)]
struct Role<'a> {
    role: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RowNote {
    role: String,
    pub source_row_key: String,
    pub provider_team_id: String,
    pub provider_athlete_id: String,
    pub capture_url: String,
    pub response_url: Option<String>,
    pub capture_sha256: String,
    pub capture_bytes: usize,
    pub fetched_at: String,
    pub row: SourceRowLocator,
    pub row_sha256: String,
}

impl RowNote {
    pub(super) fn read(evidence: &Evidence) -> StoreResult<Option<Self>> {
        let Some(raw) = &evidence.note else {
            return Ok(None);
        };
        let Ok(role) = serde_json::from_str::<Role<'_>>(raw) else {
            return Ok(None);
        };
        if role.role != Some(ROLE) {
            return Ok(None);
        }
        if raw.len() > 16 * 1024 {
            return Err(invalid(
                "current affiliation note exceeds its retained byte ceiling",
            ));
        }
        let note: Self = serde_json::from_str(raw).map_err(|source| StoreError::Decode {
            key: "roster positive row metadata".into(),
            source,
        })?;
        note.validate(evidence)?;
        Ok(Some(note))
    }

    fn validate(&self, evidence: &Evidence) -> StoreResult<()> {
        if self.role != ROLE
            || evidence.method != EvidenceMethod::Parsed
            || evidence.source.url.as_deref() != Some(self.capture_url.as_str())
            || evidence.observed_on != self.fetched_at
            || !digest(&self.capture_sha256)
            || !digest(&self.row_sha256)
            || self.capture_bytes == 0
            || self.capture_bytes > 8 * 1024 * 1024
            || self.row.byte_length == 0
            || self.row.byte_length > 64 * 1024
            || self.row.ordinal >= 20_000
            || !provider_id(&self.provider_athlete_id)
            || !provider_id(&self.provider_team_id)
            || self.capture_url.len() > 4096
            || self
                .response_url
                .as_ref()
                .is_some_and(|value| value.len() > 4096)
        {
            return Err(invalid("retained roster row metadata is inconsistent"));
        }
        self.acquired_at()?;
        let expected = format!(
            "{}#capture={}&row={}&offset={}&length={}",
            self.capture_url,
            self.capture_sha256,
            self.row.ordinal,
            self.row.byte_offset,
            self.row.byte_length,
        );
        if self.source_row_key != expected {
            return Err(invalid(
                "retained roster row key changed its capture or locator",
            ));
        }
        Ok(())
    }

    pub(super) fn acquired_at(&self) -> StoreResult<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.fetched_at)
            .map(|instant| instant.with_timezone(&Utc))
            .map_err(|_| invalid("retained roster acquisition instant is invalid"))
    }
}

pub(super) fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn provider_id(value: &str) -> bool {
    value.len() <= 20
        && !value.starts_with('0')
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u64>().is_ok_and(|value| value > 0)
}
