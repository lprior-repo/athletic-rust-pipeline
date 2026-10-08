use super::{SourceIdentity, SourceRef};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttestationQualification {
    CandidateOnly,
    IndependentPublished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AttestationWire")]
pub struct IdentityAttestation {
    pub subject: SourceIdentity,
    pub source: SourceRef,
    pub capture_sha256: String,
    pub acquired_at: String,
    pub source_family: String,
    pub upstream_producer: String,
    pub subject_locator: String,
    pub qualification: AttestationQualification,
}

#[derive(Deserialize)]
struct AttestationWire {
    subject: SourceIdentity,
    source: SourceRef,
    capture_sha256: String,
    acquired_at: String,
    source_family: String,
    upstream_producer: String,
    subject_locator: String,
    qualification: AttestationQualification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AttestationError {
    #[error("identity attestation lacks a valid bound immutable capture or producer lineage")]
    InvalidCapture,
}

impl IdentityAttestation {
    pub fn validate(&self) -> Result<(), AttestationError> {
        let valid_text = [
            &self.subject.id,
            &self.source.id,
            &self.source_family,
            &self.upstream_producer,
            &self.subject_locator,
        ]
        .iter()
        .all(|value| {
            !value.trim().is_empty()
                && value.len() <= 512
                && value.trim() == value.as_str()
                && !value.chars().any(char::is_control)
        });
        if !valid_text
            || self.capture_sha256.len() != 64
            || !self
                .capture_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || self
                .source
                .url
                .as_deref()
                .is_none_or(|value| !capture_url(value))
            || self.acquired_at.len() > 64
            || chrono::DateTime::parse_from_rfc3339(&self.acquired_at).is_err()
        {
            return Err(AttestationError::InvalidCapture);
        }
        Ok(())
    }

    pub fn is_independent_of(&self, other: &Self) -> bool {
        self.validate().is_ok()
            && other.validate().is_ok()
            && self.qualification == AttestationQualification::IndependentPublished
            && other.qualification == AttestationQualification::IndependentPublished
            && self.subject.namespace == other.subject.namespace
            && self.subject.id == other.subject.id
            && lineages_independent(self.lineage(), other.lineage())
    }

    fn lineage(&self) -> [&str; 3] {
        [
            &self.capture_sha256,
            &self.source_family,
            &self.upstream_producer,
        ]
    }

    pub(super) fn independent_lineage(&self) -> Option<IdentityLineage> {
        (self.qualification == AttestationQualification::IndependentPublished).then(|| {
            IdentityLineage {
                capture_sha256: self.capture_sha256.clone(),
                source_family: self.source_family.clone(),
                upstream_producer: self.upstream_producer.clone(),
            }
        })
    }
}

#[derive(PartialEq, Eq)]
pub(super) struct IdentityLineage {
    capture_sha256: String,
    source_family: String,
    upstream_producer: String,
}

impl IdentityLineage {
    pub(super) fn is_independent_of(&self, other: &Self) -> bool {
        lineages_independent(
            [
                &self.capture_sha256,
                &self.source_family,
                &self.upstream_producer,
            ],
            [
                &other.capture_sha256,
                &other.source_family,
                &other.upstream_producer,
            ],
        )
    }

    pub(super) fn matches(&self, claim: &IdentityAttestation) -> bool {
        [
            &self.capture_sha256,
            &self.source_family,
            &self.upstream_producer,
        ]
        .into_iter()
        .zip(claim.lineage())
        .all(|(left, right)| left.eq_ignore_ascii_case(right))
    }
}

fn lineages_independent(first: [&str; 3], second: [&str; 3]) -> bool {
    first
        .into_iter()
        .zip(second)
        .all(|(left, right)| !left.eq_ignore_ascii_case(right))
}

fn capture_url(value: &str) -> bool {
    value.len() <= 4096
        && !value.chars().any(char::is_whitespace)
        && url::Url::parse(value).is_ok_and(|url| {
            matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
        })
}

impl TryFrom<AttestationWire> for IdentityAttestation {
    type Error = AttestationError;
    fn try_from(row: AttestationWire) -> Result<Self, Self::Error> {
        let claim = Self {
            subject: row.subject,
            source: row.source,
            capture_sha256: row.capture_sha256,
            acquired_at: row.acquired_at,
            source_family: row.source_family,
            upstream_producer: row.upstream_producer,
            subject_locator: row.subject_locator,
            qualification: row.qualification,
        };
        claim.validate()?;
        Ok(claim)
    }
}
