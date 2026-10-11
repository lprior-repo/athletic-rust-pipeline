use super::engine_valid::{refuse_date, refuse_id, valid_hex64, MAX_LOGICAL_ID_BYTES};
use super::*;
use crate::DomainError;

pub const MAX_SOURCE_KEY_BYTES: usize = 256;
pub const MAX_FAMILY_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Corroboration {
    Independent,
    Syndicated { family: String },
}

impl Corroboration {
    pub fn independent() -> Self {
        Self::Independent
    }
    pub fn syndicated(family: String) -> Result<Self, DomainError> {
        if family.trim().is_empty()
            || family.len() > MAX_FAMILY_BYTES
            || family.bytes().any(|b| b == 0)
        {
            return Err(DomainError::Unsupported { field: "family" });
        }
        Ok(Self::Syndicated { family })
    }
    pub fn is_independent(&self) -> bool {
        matches!(self, Self::Independent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PerformanceClaim {
    id: String,
    athlete_id: String,
    meet_id: String,
    event: EventKind,
    date: String,
    source_key: String,
    capture_digest: String,
    corroboration: Corroboration,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPerformance {
    id: String,
    athlete_id: String,
    meet_id: String,
    event: EventKind,
    date: String,
    source_key: String,
    capture_digest: String,
    corroboration: Corroboration,
}

fn performance_id(
    athlete: &str,
    meet: &str,
    event: &EventKind,
    date: &str,
    source_key: &str,
) -> String {
    Id::<tag::Performance>::mint(
        "perf",
        &[athlete, meet, event.stable_key().as_ref(), date, source_key],
    )
    .to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceRequest {
    pub athlete_id: String,
    pub meet_id: String,
    pub event: EventKind,
    pub date: String,
    pub source_key: String,
    pub capture_digest: String,
    pub corroboration: Corroboration,
}

impl PerformanceClaim {
    pub fn new(request: PerformanceRequest) -> Result<Self, DomainError> {
        refuse_id("athlete_id", &request.athlete_id)?;
        refuse_id("meet_id", &request.meet_id)?;
        refuse_date("date", &request.date)?;
        if request.source_key.is_empty() || request.source_key.len() > MAX_SOURCE_KEY_BYTES {
            return Err(DomainError::OutOfRange {
                field: "source_key",
            });
        }
        if !valid_hex64(&request.capture_digest) {
            return Err(DomainError::Unsupported {
                field: "capture_digest",
            });
        }
        if let Corroboration::Syndicated { family } = &request.corroboration {
            if family.trim().is_empty() || family.len() > MAX_FAMILY_BYTES {
                return Err(DomainError::Unsupported { field: "family" });
            }
        }
        let id = performance_id(
            &request.athlete_id,
            &request.meet_id,
            &request.event,
            &request.date,
            &request.source_key,
        );
        if id.len() > MAX_LOGICAL_ID_BYTES {
            return Err(DomainError::OutOfRange { field: "id" });
        }
        Ok(Self {
            id,
            athlete_id: request.athlete_id,
            meet_id: request.meet_id,
            event: request.event,
            date: request.date,
            source_key: request.source_key,
            capture_digest: request.capture_digest,
            corroboration: request.corroboration,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn athlete_id(&self) -> &str {
        &self.athlete_id
    }
    pub fn meet_id(&self) -> &str {
        &self.meet_id
    }
    pub fn event(&self) -> &EventKind {
        &self.event
    }
    pub fn date(&self) -> &str {
        &self.date
    }
    pub fn source_key(&self) -> &str {
        &self.source_key
    }
    pub fn capture_digest(&self) -> &str {
        &self.capture_digest
    }
    pub fn corroboration(&self) -> &Corroboration {
        &self.corroboration
    }
    pub fn same_event(&self, other: &Self) -> bool {
        self.event == other.event
    }
}

impl<'de> Deserialize<'de> for PerformanceClaim {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawPerformance::deserialize(deserializer)?;
        let rebuilt = Self::new(PerformanceRequest {
            athlete_id: raw.athlete_id.clone(),
            meet_id: raw.meet_id.clone(),
            event: raw.event.clone(),
            date: raw.date.clone(),
            source_key: raw.source_key.clone(),
            capture_digest: raw.capture_digest.clone(),
            corroboration: raw.corroboration.clone(),
        })
        .map_err(serde::de::Error::custom)?;
        if rebuilt.id != raw.id {
            return Err(serde::de::Error::custom(
                "performance id does not match its fields",
            ));
        }
        Ok(rebuilt)
    }
}
