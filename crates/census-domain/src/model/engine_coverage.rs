use super::engine_valid::{valid_detail, valid_hex64, MAX_LOGICAL_ID_BYTES};
use super::*;
use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageState {
    Pending,
    Complete,
    Blocked { kind: AccessBlockKind },
    RetainedGap,
}

impl CoverageState {
    pub const fn is_terminal(self) -> bool {
        match self {
            Self::Pending => false,
            Self::Complete | Self::Blocked { .. } | Self::RetainedGap => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoverageObligation {
    id: String,
    scope: CoverageScope,
    subject: String,
    season: String,
    revision: u32,
    state: CoverageState,
    evidence_digest: Option<String>,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawObligation {
    id: String,
    scope: CoverageScope,
    subject: String,
    season: String,
    revision: u32,
    state: CoverageState,
    evidence_digest: Option<String>,
    detail: String,
}

fn check_new(
    id: &str,
    subject: &str,
    season: &str,
    revision: u32,
    detail: &str,
) -> Result<(), DomainError> {
    if id.is_empty() || id.len() > MAX_LOGICAL_ID_BYTES {
        return Err(DomainError::OutOfRange { field: "id" });
    }
    if subject.is_empty() || subject.len() > MAX_LOGICAL_ID_BYTES {
        return Err(DomainError::OutOfRange { field: "subject" });
    }
    if season.trim().is_empty() || season.len() > MAX_LOGICAL_ID_BYTES {
        return Err(DomainError::OutOfRange { field: "season" });
    }
    if revision == 0 {
        return Err(DomainError::OutOfRange { field: "revision" });
    }
    if !id.contains(':') {
        return Err(DomainError::Unsupported { field: "id" });
    }
    if !valid_detail(detail) {
        return Err(DomainError::OutOfRange { field: "detail" });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingObligation {
    pub id: String,
    pub scope: CoverageScope,
    pub subject: String,
    pub season: String,
    pub revision: u32,
    pub detail: String,
}

impl CoverageObligation {
    pub fn pending(request: PendingObligation) -> Result<Self, DomainError> {
        check_new(
            &request.id,
            &request.subject,
            &request.season,
            request.revision,
            &request.detail,
        )?;
        Ok(Self {
            id: request.id,
            scope: request.scope,
            subject: request.subject,
            season: request.season,
            revision: request.revision,
            state: CoverageState::Pending,
            evidence_digest: None,
            detail: request.detail,
        })
    }
    pub fn complete(&self, evidence_digest: String) -> Result<Self, DomainError> {
        if self.state != CoverageState::Pending {
            return Err(DomainError::Unsupported { field: "state" });
        }
        if !valid_hex64(&evidence_digest) {
            return Err(DomainError::Unsupported {
                field: "evidence_digest",
            });
        }
        Ok(Self {
            id: self.id.clone(),
            scope: self.scope,
            subject: self.subject.clone(),
            season: self.season.clone(),
            revision: self.revision,
            state: CoverageState::Complete,
            evidence_digest: Some(evidence_digest),
            detail: self.detail.clone(),
        })
    }
    pub fn block(&self, kind: AccessBlockKind, detail: String) -> Result<Self, DomainError> {
        if self.state != CoverageState::Pending {
            return Err(DomainError::Unsupported { field: "state" });
        }
        if !valid_detail(&detail) {
            return Err(DomainError::OutOfRange { field: "detail" });
        }
        Ok(Self {
            id: self.id.clone(),
            scope: self.scope,
            subject: self.subject.clone(),
            season: self.season.clone(),
            revision: self.revision,
            state: CoverageState::Blocked { kind },
            evidence_digest: None,
            detail,
        })
    }
    pub fn retain_gap(&self, detail: String) -> Result<Self, DomainError> {
        if self.state != CoverageState::Pending {
            return Err(DomainError::Unsupported { field: "state" });
        }
        if !valid_detail(&detail) {
            return Err(DomainError::OutOfRange { field: "detail" });
        }
        Ok(Self {
            id: self.id.clone(),
            scope: self.scope,
            subject: self.subject.clone(),
            season: self.season.clone(),
            revision: self.revision,
            state: CoverageState::RetainedGap,
            evidence_digest: None,
            detail,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn scope(&self) -> CoverageScope {
        self.scope
    }
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn season(&self) -> &str {
        &self.season
    }
    pub fn revision(&self) -> u32 {
        self.revision
    }
    pub fn state(&self) -> &CoverageState {
        &self.state
    }
    pub fn evidence_digest(&self) -> Option<&str> {
        self.evidence_digest.as_deref()
    }
    pub fn detail(&self) -> &str {
        &self.detail
    }
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }
}

impl<'de> Deserialize<'de> for CoverageObligation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawObligation::deserialize(deserializer)?;
        check_new(
            &raw.id,
            &raw.subject,
            &raw.season,
            raw.revision,
            &raw.detail,
        )
        .map_err(serde::de::Error::custom)?;
        match (&raw.state, &raw.evidence_digest) {
            (CoverageState::Complete, Some(digest)) => {
                if !valid_hex64(digest) {
                    return Err(serde::de::Error::custom(
                        "evidence_digest is not 64-char hex",
                    ));
                }
            }
            (CoverageState::Complete, None) => {
                return Err(serde::de::Error::custom(
                    "complete obligation needs evidence",
                ));
            }
            (_, Some(_)) => {
                return Err(serde::de::Error::custom(
                    "only complete obligations carry evidence",
                ));
            }
            (CoverageState::Pending, _)
            | (CoverageState::Blocked { .. }, _)
            | (CoverageState::RetainedGap, _) => {}
        }
        if raw.id.len() > MAX_LOGICAL_ID_BYTES {
            return Err(serde::de::Error::custom(
                "obligation id is outside its permitted range",
            ));
        }
        Ok(Self {
            id: raw.id,
            scope: raw.scope,
            subject: raw.subject,
            season: raw.season,
            revision: raw.revision,
            state: raw.state,
            evidence_digest: raw.evidence_digest,
            detail: raw.detail,
        })
    }
}
