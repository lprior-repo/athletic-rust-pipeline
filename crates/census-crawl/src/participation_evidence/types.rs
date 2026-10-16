use super::{hash, invalid};
use census_domain::model::{GradYear, ProfileDate, SchoolYear, SourceIdentity, StoreIdentity};
use census_store::StoreResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParticipationEvidenceRefV1(String);

impl ParticipationEvidenceRefV1 {
    pub fn parse(value: &str) -> StoreResult<Self> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid("participation packet reference is not a SHA256"));
        }
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSubjectV1 {
    pub(super) document_namespace: String,
    pub(super) native_document_id: String,
    pub(super) document_url: String,
    pub(super) body_sha256: String,
    pub(super) locator: String,
    pub(super) name: String,
    pub(super) school: String,
}

impl DocumentSubjectV1 {
    pub fn document_namespace(&self) -> &str {
        &self.document_namespace
    }
    pub fn native_document_id(&self) -> &str {
        &self.native_document_id
    }
    pub fn document_url(&self) -> &str {
        &self.document_url
    }
    pub fn body_sha256(&self) -> &str {
        &self.body_sha256
    }
    pub fn locator(&self) -> &str {
        &self.locator
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn school(&self) -> &str {
        &self.school
    }
    pub fn reference(&self) -> String {
        hash(
            format!(
                "document_subject_v1\n{}\n{}\n{}\n{}\n{}",
                self.document_namespace,
                self.native_document_id,
                self.document_url,
                self.body_sha256,
                self.locator
            )
            .as_bytes(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubjectOwnershipV1 {
    ProviderLink(SourceIdentity),
    DocumentOnly(DocumentSubjectV1),
}

#[derive(Debug, Clone)]
pub struct IndependentParticipationProofV1 {
    pub(super) capture: String,
    pub(super) interpretation: String,
    pub(super) locator: String,
    pub(super) acquired_at: DateTime<Utc>,
}

impl IndependentParticipationProofV1 {
    pub fn capture(&self) -> &str {
        &self.capture
    }
    pub fn interpretation(&self) -> &str {
        &self.interpretation
    }
    pub fn locator(&self) -> &str {
        &self.locator
    }
    pub fn acquired_at(&self) -> DateTime<Utc> {
        self.acquired_at
    }
}

#[derive(Debug)]
pub struct VerifiedIndependentParticipationV1 {
    pub(super) store_identity: StoreIdentity,
    pub(super) snapshot_sequence: u64,
    pub(super) reference: ParticipationEvidenceRefV1,
    pub(super) subject: DocumentSubjectV1,
    pub(super) ownership: SubjectOwnershipV1,
    pub(super) day: ProfileDate,
    pub(super) school_year: SchoolYear,
    pub(super) cohort: GradYear,
    pub(super) competition: String,
    pub(super) venue: String,
    pub(super) season_scope: String,
    pub(super) status: IndependentParticipationProofV1,
    pub(super) season: IndependentParticipationProofV1,
}

impl VerifiedIndependentParticipationV1 {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store_identity
    }
    pub fn snapshot_sequence(&self) -> u64 {
        self.snapshot_sequence
    }
    pub fn reference(&self) -> &ParticipationEvidenceRefV1 {
        &self.reference
    }
    pub fn subject(&self) -> &DocumentSubjectV1 {
        &self.subject
    }
    pub fn ownership(&self) -> &SubjectOwnershipV1 {
        &self.ownership
    }
    pub fn day(&self) -> &ProfileDate {
        &self.day
    }
    pub fn school_year(&self) -> SchoolYear {
        self.school_year
    }
    pub fn cohort(&self) -> GradYear {
        self.cohort
    }
    pub fn competition(&self) -> &str {
        &self.competition
    }
    pub fn venue(&self) -> &str {
        &self.venue
    }
    pub fn season_scope(&self) -> &str {
        &self.season_scope
    }
    pub fn status_proof(&self) -> &IndependentParticipationProofV1 {
        &self.status
    }
    pub fn season_proof(&self) -> &IndependentParticipationProofV1 {
        &self.season
    }
}
