use super::{request, ReviewedIdentityRequestV1};
use crate::public_identity::PublicIdentityError;
use crate::StoreResult;

use census_domain::public_identity_history::{
    CandidateDecisionReference, HistoryRecord, HistoryRequest, PreparedHistoryRecord,
    PublicAthleteId,
};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::public_identity) struct ReviewedWitness {
    pub version: u32,
    pub operation: String,
    pub kind: request::ReviewedKind,
    pub case_id: String,
    pub request_digest: String,
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub snapshot_sequence: u64,
    pub record_digest: String,
    pub decision_id: String,
    pub decision_digest: String,
    pub introduced_public_ids: Vec<String>,
    pub authority_context: String,
}

impl ReviewedWitness {
    #[cfg(test)]
    pub(super) fn new(
        operation: &str,
        request: &ReviewedIdentityRequestV1,
        snapshot: &crate::StoreSnapshot<'_>,
        prepared: &PreparedHistoryRecord,
        decision: &census_domain::model::AppliedAthleteIdentity,
        context: &str,
    ) -> StoreResult<Self> {
        Ok(Self {
            version: 3,
            operation: operation.into(),
            kind: request.kind(),
            case_id: request.case_id().into(),
            request_digest: request::digest(
                &prepared.uncommitted_record.installation,
                operation,
                request,
            )?,
            evidence_generation: snapshot.evidence_generation(),
            derived_generation: snapshot.derived_generation(),
            snapshot_sequence: snapshot.sequence(),
            record_digest: prepared.uncommitted_record.digest.clone(),
            decision_id: decision.id.clone(),
            decision_digest: census_domain::model::serialized_digest(decision)?,
            introduced_public_ids: prepared
                .established_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            authority_context: context.into(),
        })
    }

    pub(in crate::public_identity) fn validate(
        &self,
        version: PublicIdentityQualificationVersion,
        prepared: &PreparedHistoryRecord,
    ) -> StoreResult<()> {
        if version != PublicIdentityQualificationVersion::RetainedAuthorityContextV2
            || self.version != 3
            || !crate::journal::is_object_id(&self.authority_context)
        {
            return Err(
                PublicIdentityError::Corrupt("reviewed witness qualification binding").into(),
            );
        }
        let record = &prepared.uncommitted_record;
        let request = ReviewedIdentityRequestV1::from_history(&self.case_id, &record.request)?;
        let expected_ids: Vec<_> = prepared
            .established_ids
            .iter()
            .map(PublicAthleteId::as_str)
            .collect();
        if self.kind != request.kind()
            || self.record_digest != record.digest
            || self.decision_id != evidence(&record.request)?.application_id
            || self.request_digest
                != request::digest(&record.installation, &self.operation, &request)?
            || self.introduced_public_ids != expected_ids
        {
            return Err(PublicIdentityError::Corrupt("reviewed commit witness binding").into());
        }
        Ok(())
    }

    pub(in crate::public_identity) fn verify_rows(
        &self,
        reader: &impl crate::public_identity::retained::HistoricalRead,
        identity: &census_domain::model::StoreIdentity,
        record: &HistoryRecord,
    ) -> StoreResult<()> {
        super::rows::verify(self, reader, identity, record)
    }
}

pub(super) fn evidence(request: &HistoryRequest) -> StoreResult<&CandidateDecisionReference> {
    match request {
        HistoryRequest::Merge { evidence, .. } => Ok(evidence),
        HistoryRequest::Split(split) => Ok(&split.evidence),
        HistoryRequest::Revoke(revocation) => Ok(&revocation.evidence),
        _ => Err(PublicIdentityError::Corrupt("unsupported reviewed evidence").into()),
    }
}
