use super::{Result, RosterAuthorityError, SourceRowLocator};
use crate::model::{Evidence, EvidenceMethod};
use chrono::{DateTime, Utc};
use serde::Deserialize;

const ROLE: &str = "roster_positive_row_metadata_v1";

#[derive(Deserialize)]
struct Role<'a> {
    role: Option<&'a str>,
}

#[derive(Deserialize)]
struct RowFields {
    ordinal: u32,
    byte_offset: usize,
    byte_length: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterMemberObservationV1 {
    role: String,
    source_row_key: String,
    provider_team_id: String,
    provider_athlete_id: String,
    capture_url: String,
    response_url: Option<String>,
    capture_sha256: String,
    capture_bytes: usize,
    fetched_at: String,
    #[serde(deserialize_with = "row_locator")]
    row: SourceRowLocator,
    row_sha256: String,
}

impl RosterMemberObservationV1 {
    pub fn read(evidence: &Evidence) -> Result<Option<Self>> {
        let Some(raw) = &evidence.note else {
            return Ok(None);
        };
        if raw.len() > 16 * 1024 {
            return Err(RosterAuthorityError::Invalid(
                "current affiliation note exceeds its retained byte ceiling",
            ));
        }
        let Ok(role) = serde_json::from_str::<Role<'_>>(raw) else {
            return Ok(None);
        };
        if role.role != Some(ROLE) {
            return Ok(None);
        }
        let note: Self = serde_json::from_str(raw).map_err(RosterAuthorityError::Decode)?;
        note.validate(evidence)?;
        Ok(Some(note))
    }

    fn validate(&self, evidence: &Evidence) -> Result<()> {
        if self.role != ROLE
            || evidence.method != EvidenceMethod::Parsed
            || evidence.source.url.as_deref() != Some(self.capture_url.as_str())
            || evidence.observed_on != self.fetched_at
            || !digest(&self.capture_sha256)
            || !digest(&self.row_sha256)
            || self.capture_bytes == 0
            || self.capture_bytes > 8 * 1024 * 1024
            || self.row.byte_length() == 0
            || self.row.byte_length() > 64 * 1024
            || self.row.ordinal() >= 20_000
            || !provider_id(&self.provider_athlete_id)
            || !provider_id(&self.provider_team_id)
            || self.capture_url.len() > 4096
            || self
                .response_url
                .as_ref()
                .is_some_and(|value| value.len() > 4096)
        {
            return Err(RosterAuthorityError::Invalid(
                "retained roster row metadata is inconsistent",
            ));
        }
        self.acquired_at()?;
        let expected = format!(
            "{}#capture={}&row={}&offset={}&length={}",
            self.capture_url,
            self.capture_sha256,
            self.row.ordinal(),
            self.row.byte_offset(),
            self.row.byte_length(),
        );
        if self.source_row_key != expected {
            return Err(RosterAuthorityError::Invalid(
                "retained roster row key changed its capture or locator",
            ));
        }
        Ok(())
    }

    pub fn source_row_key(&self) -> &str {
        &self.source_row_key
    }

    pub fn provider_team_id(&self) -> &str {
        &self.provider_team_id
    }

    pub fn provider_athlete_id(&self) -> &str {
        &self.provider_athlete_id
    }

    pub fn capture_url(&self) -> &str {
        &self.capture_url
    }

    pub fn response_url(&self) -> Option<&str> {
        self.response_url.as_deref()
    }

    pub fn capture_sha256(&self) -> &str {
        &self.capture_sha256
    }

    pub fn capture_bytes(&self) -> usize {
        self.capture_bytes
    }

    pub fn fetched_at(&self) -> &str {
        &self.fetched_at
    }

    pub fn row(&self) -> &SourceRowLocator {
        &self.row
    }

    pub fn row_sha256(&self) -> &str {
        &self.row_sha256
    }

    pub fn acquired_at(&self) -> Result<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.fetched_at)
            .map(|instant| instant.with_timezone(&Utc))
            .map_err(|_| {
                RosterAuthorityError::Invalid("retained roster acquisition instant is invalid")
            })
    }
}

fn row_locator<'de, D>(decoder: D) -> std::result::Result<SourceRowLocator, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let fields = RowFields::deserialize(decoder)?;
    Ok(SourceRowLocator::new(
        fields.ordinal,
        fields.byte_offset,
        fields.byte_length,
    ))
}

fn digest(value: &str) -> bool {
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
#[cfg(test)]
mod tests;
