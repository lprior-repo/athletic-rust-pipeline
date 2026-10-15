use crate::model::{
    serialized_digest, AcceptedAthleteIdentity, AppliedAthleteIdentity, AppliedIdentityKind,
    AthleteCandidateId,
};
use serde::{Deserialize, Serialize};

mod correction;

use super::{HistoryError, ReplayLimits};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateEvidenceKind {
    SourceBound,
    SamePerson,
    Correction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CandidateDecisionReference {
    pub application_id: String,
    pub evidence_digest: String,
    pub kind: CandidateEvidenceKind,
    pub members: Vec<AthleteCandidateId>,
}

impl CandidateDecisionReference {
    pub fn from_accepted(
        application: &AcceptedAthleteIdentity,
        limits: ReplayLimits,
    ) -> Result<Self, HistoryError> {
        Self::from_record(application.record(), limits)
    }

    pub fn matches_applied(
        &self,
        record: &AppliedAthleteIdentity,
        limits: ReplayLimits,
    ) -> Result<bool, HistoryError> {
        Ok(self == &Self::from_record(record, limits)?)
    }

    fn from_record(
        record: &AppliedAthleteIdentity,
        limits: ReplayLimits,
    ) -> Result<Self, HistoryError> {
        super::budget::accepted_reference(record, limits)?;
        let kind = match record.kind {
            AppliedIdentityKind::SourceBound => CandidateEvidenceKind::SourceBound,
            AppliedIdentityKind::SamePerson => CandidateEvidenceKind::SamePerson,
            AppliedIdentityKind::DifferentPerson => return Err(HistoryError::EvidenceMismatch),
        };
        Self::from_kind(record, kind)
    }

    fn from_kind(
        record: &AppliedAthleteIdentity,
        kind: CandidateEvidenceKind,
    ) -> Result<Self, HistoryError> {
        Ok(Self {
            application_id: record.id.clone(),
            evidence_digest: serialized_digest(&(
                "accepted-athlete-identity-reference-v1",
                &record.id,
                record.policy,
                record.kind,
                &record.members,
                &record.canonical_id,
                &record.case_id,
                &record.verdict_digest,
            ))
            .map_err(|error| HistoryError::Encoding(error.to_string()))?,
            kind,
            members: record
                .members
                .iter()
                .map(|member| member.subject.clone())
                .collect(),
        })
    }
}
