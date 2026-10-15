use super::{
    claims, consume, prepare, ApprovedLocalAdjudicationV1, LocalAdjudicationCommitOutcomeV1,
};
use crate::public_identity::{
    codec,
    retained::{Read, Revision, Rows},
    witness, PublicIdentityError,
};
use crate::StoreResult;
use census_domain::model::{serialized_digest, StoreIdentity};
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV2;
use census_domain::public_identity_history::{
    prepare_next, HistoryProjection, HistoryRecord, PreparedHistoryRecord, PublicAthleteId,
    RegistryHead, ReplayLimits,
};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::public_identity) struct LocalAdjudicationWitnessV1 {
    pub version: u32,
    pub operation: String,
    pub request_digest: String,
    pub actor: LocalActor,
    pub adjudication: LocalIdentityAdjudicationRecordV2,
    pub record_digest: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::public_identity) enum LocalActor {
    LocalOperator,
}

impl LocalAdjudicationWitnessV1 {
    pub(super) fn new(approved: ApprovedLocalAdjudicationV1) -> Self {
        Self {
            version: 5,
            operation: approved.record.operation.clone(),
            request_digest: approved.approval_digest,
            actor: LocalActor::LocalOperator,
            record_digest: approved.history.uncommitted_record.digest,
            adjudication: approved.record,
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
            || self.version != 5
            || self.operation != self.adjudication.operation
            || self.record_digest != record.digest
            || self.adjudication.store != record.installation
            || self.adjudication.expected_head != record.previous
            || self.request_digest
                != consume::approval_digest(&consume::approval_bytes(&self.adjudication)?)
        {
            return Err(PublicIdentityError::Corrupt("manual witness binding").into());
        }
        reader.revision()?.includes(Revision {
            evidence_generation: self.adjudication.evidence_generation,
            derived_generation: self.adjudication.derived_generation,
            snapshot_sequence: self.adjudication.snapshot_sequence,
        })?;
        self.verify_authority(reader, &record.installation, previous, prepared, records)?;
        let key = witness::receipt_key(&self.operation);
        let raw = reader
            .row(Rows::Receipt, &key)?
            .ok_or(PublicIdentityError::Corrupt("missing manual receipt"))?;
        let receipt: crate::Receipt = codec::decode(&key, &raw)?;
        if receipt.operation != key
            || receipt.digest != serialized_digest(self)?
            || receipt.at != self.adjudication.observed_at
            || receipt.appended != 1
            || receipt.credited
        {
            return Err(PublicIdentityError::Corrupt("manual receipt binding").into());
        }
        Ok(())
    }

    fn verify_authority(
        &self,
        reader: &impl Read,
        identity: &StoreIdentity,
        previous: &HistoryProjection,
        prepared: &PreparedHistoryRecord,
        records: &[HistoryRecord],
    ) -> StoreResult<()> {
        let record = &self.adjudication;
        let inputs = reader.context(&record.authority_context, identity)?;
        let members = prepare::members(&inputs, previous, &record.request.canonical)?;
        if members != record.members {
            return Err(PublicIdentityError::Corrupt("manual historical member binding").into());
        }
        prepare::validate_case(&inputs, &record.request, &members)?;
        prepare::validate_prior(&record.request, records)?;
        super::fence::validate_superseded(reader, record)?;
        reader
            .affected_context(&record.affected_context)?
            .validate(&inputs, record)?;
        let claims = claims::inventory(&inputs, &members)?;
        let request = record
            .history_request(&claims)
            .map_err(PublicIdentityError::from)?;
        let replayed = prepare_next(identity, previous.head(), request, ReplayLimits::SMALL)
            .map_err(PublicIdentityError::from)?;
        if &replayed != prepared {
            return Err(
                PublicIdentityError::Corrupt("manual historical correction binding").into(),
            );
        }
        Ok(())
    }

    pub(in crate::public_identity) fn outcome(
        &self,
        record: &HistoryRecord,
        replayed: bool,
    ) -> StoreResult<LocalAdjudicationCommitOutcomeV1> {
        let prepared = prepare_next(
            &record.installation,
            &record.previous,
            record.request.clone(),
            ReplayLimits::SMALL,
        )
        .map_err(PublicIdentityError::from)?;
        let ids: Vec<PublicAthleteId> = prepared.established_ids;
        Ok(LocalAdjudicationCommitOutcomeV1 {
            introduced_public_ids: ids,
            head: RegistryHead {
                sequence: record.sequence,
                digest: record.digest.clone(),
            },
            replayed,
        })
    }
}
