use super::{consume, MembershipWithholdingOutcomeV1};
use crate::public_identity::{
    codec,
    local_adjudication::{claims, prepare::members},
    retained::{Read, Revision, Rows},
    witness, PublicIdentityError,
};
use crate::StoreResult;
use census_domain::model::serialized_digest;
use census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV1;
use census_domain::public_identity_history::{
    HistoryProjection, HistoryRecord, PreparedHistoryRecord, RegistryHead,
};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::public_identity) struct MembershipWithholdingWitnessV1 {
    pub version: u32,
    pub operation: String,
    pub request_digest: String,
    actor: LocalActor,
    pub adjudication: LocalMembershipWithholdingRecordV1,
    pub record_digest: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LocalActor {
    LocalOperator,
}

impl MembershipWithholdingWitnessV1 {
    pub(super) fn new(
        record: LocalMembershipWithholdingRecordV1,
        history: &PreparedHistoryRecord,
        approval_digest: String,
    ) -> Self {
        Self {
            version: 7,
            operation: record.operation.clone(),
            request_digest: approval_digest,
            actor: LocalActor::LocalOperator,
            adjudication: record,
            record_digest: history.uncommitted_record.digest.clone(),
        }
    }
    pub(in crate::public_identity) fn verify(
        &self,
        reader: &impl Read,
        version: PublicIdentityQualificationVersion,
        previous: &HistoryProjection,
        prepared: &PreparedHistoryRecord,
        records: &[HistoryRecord],
    ) -> StoreResult<()> {
        let record = &prepared.uncommitted_record;
        if version != PublicIdentityQualificationVersion::RetainedAuthorityContextV2
            || self.version != 7
            || self.operation != self.adjudication.operation
            || self.record_digest != record.digest
            || self.adjudication.store != record.installation
            || self.adjudication.expected_head != record.previous
            || self.request_digest
                != consume::approval_digest(&consume::approval_bytes(&self.adjudication)?)
        {
            return Err(PublicIdentityError::Corrupt("withholding witness binding").into());
        }
        reader.revision()?.includes(Revision {
            evidence_generation: self.adjudication.evidence_generation,
            derived_generation: self.adjudication.derived_generation,
            snapshot_sequence: self.adjudication.snapshot_sequence,
        })?;
        if rebuild(reader, &self.adjudication, previous, records)? != *prepared {
            return Err(
                PublicIdentityError::Corrupt("withholding historical correction binding").into(),
            );
        }
        let key = witness::receipt_key(&self.operation);
        let raw = reader
            .row(Rows::Receipt, &key)?
            .ok_or(PublicIdentityError::Corrupt("missing withholding receipt"))?;
        let receipt: crate::Receipt = codec::decode(&key, &raw)?;
        if receipt.operation != key
            || receipt.digest != serialized_digest(self)?
            || receipt.at != self.adjudication.observed_at
            || receipt.appended != 1
            || receipt.credited
        {
            return Err(PublicIdentityError::Corrupt("withholding receipt binding").into());
        }
        Ok(())
    }
    pub(in crate::public_identity) fn outcome(
        &self,
        record: &HistoryRecord,
        replayed: bool,
    ) -> StoreResult<MembershipWithholdingOutcomeV1> {
        if self.record_digest != record.digest {
            return Err(PublicIdentityError::Corrupt("withholding outcome binding").into());
        }
        Ok(MembershipWithholdingOutcomeV1 {
            head: RegistryHead {
                sequence: record.sequence,
                digest: record.digest.clone(),
            },
            withheld_memberships: self
                .adjudication
                .request
                .expected_withheld_memberships
                .clone(),
            replayed,
        })
    }
}

pub(super) fn rebuild(
    reader: &impl Read,
    record: &LocalMembershipWithholdingRecordV1,
    previous: &HistoryProjection,
    records: &[HistoryRecord],
) -> StoreResult<PreparedHistoryRecord> {
    let inputs = reader.context(&record.authority_context, &record.store)?;
    let members = members(&inputs, previous, &record.request.canonical)?;
    if members != record.members {
        return Err(PublicIdentityError::Corrupt("withholding historical member binding").into());
    }
    super::fence::validate(reader, record, records)?;
    reader
        .affected_context(&record.affected_context)?
        .validate_withholding(&inputs, record)?;
    let inventory = claims::membership_inventory(&inputs, &members, previous, records)?;
    super::prepare::prepare_history(record, &inventory, previous)
}
