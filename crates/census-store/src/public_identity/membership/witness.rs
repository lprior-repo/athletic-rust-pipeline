use super::{request, SourceCandidateMembershipOutcomeV1, SourceCandidateMembershipRequestV1};
use crate::public_identity::{
    authority, codec,
    retained::{Read, Revision, Rows},
    witness, PublicIdentityError,
};
use crate::StoreResult;
use census_domain::model::{serialized_digest, AppliedAthleteIdentity};
use census_domain::public_identity_history::{
    HistoryProjection, HistoryRecord, PreparedHistoryRecord, RegistryHead,
};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::public_identity) struct SourceCandidateMembershipWitnessV1 {
    pub version: u32,
    pub operation: String,
    pub request: SourceCandidateMembershipRequestV1,
    pub request_digest: String,
    #[serde(deserialize_with = "head")]
    pub expected_head: RegistryHead,
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub snapshot_sequence: u64,
    pub authority_context: String,
    pub decision_id: String,
    pub decision_digest: String,
    pub record_digest: String,
    #[serde(deserialize_with = "head")]
    pub resulting_head: RegistryHead,
    #[serde(deserialize_with = "empty_ids")]
    pub introduced_public_ids: Vec<String>,
}

impl SourceCandidateMembershipWitnessV1 {
    pub(super) fn new(
        operation: &str,
        request: &SourceCandidateMembershipRequestV1,
        snapshot: &crate::StoreSnapshot<'_>,
        head: &RegistryHead,
        prepared: &PreparedHistoryRecord,
        decision: &AppliedAthleteIdentity,
        context: &str,
    ) -> StoreResult<Self> {
        if !prepared.established_ids.is_empty() {
            return Err(PublicIdentityError::Corrupt("membership introduced public IDs").into());
        }
        Ok(Self {
            version: 6,
            operation: operation.into(),
            request: request.clone(),
            request_digest: request::digest(
                &prepared.uncommitted_record.installation,
                operation,
                request,
            )?,
            expected_head: head.clone(),
            evidence_generation: snapshot.evidence_generation(),
            derived_generation: snapshot.derived_generation(),
            snapshot_sequence: snapshot.sequence(),
            authority_context: context.into(),
            decision_id: decision.id.clone(),
            decision_digest: serialized_digest(decision)?,
            record_digest: prepared.uncommitted_record.digest.clone(),
            resulting_head: prepared.next_head.clone(),
            introduced_public_ids: Vec::new(),
        })
    }

    pub(in crate::public_identity) fn outcome(
        &self,
        record: &HistoryRecord,
        replayed: bool,
    ) -> StoreResult<SourceCandidateMembershipOutcomeV1> {
        if self.record_digest != record.digest || !self.introduced_public_ids.is_empty() {
            return Err(PublicIdentityError::Corrupt("membership outcome binding").into());
        }
        Ok(SourceCandidateMembershipOutcomeV1 {
            owner: self.request.owner.clone(),
            head: RegistryHead {
                sequence: record.sequence,
                digest: record.digest.clone(),
            },
            replayed,
        })
    }

    pub(in crate::public_identity) fn verify(
        &self,
        reader: &impl Read,
        version: PublicIdentityQualificationVersion,
        previous: &HistoryProjection,
        prepared: &PreparedHistoryRecord,
    ) -> StoreResult<()> {
        let record = &prepared.uncommitted_record;
        if version != PublicIdentityQualificationVersion::RetainedAuthorityContextV2
            || self.version != 6
            || !crate::journal::is_object_id(&self.authority_context)
            || self.expected_head != *previous.head()
            || self.resulting_head != prepared.next_head
            || self.record_digest != record.digest
            || !self.introduced_public_ids.is_empty()
            || !prepared.established_ids.is_empty()
            || self.request_digest
                != request::digest(&record.installation, &self.operation, &self.request)?
        {
            return Err(PublicIdentityError::Corrupt("membership witness binding").into());
        }
        reader.revision()?.includes(Revision {
            evidence_generation: self.evidence_generation,
            derived_generation: self.derived_generation,
            snapshot_sequence: self.snapshot_sequence,
        })?;
        let decision = self.decision(reader)?;
        let inputs = reader.context(&self.authority_context, &record.installation)?;
        let accepted =
            authority::accept_reviewed(&inputs, &self.request.case_id, &decision.observed_at)?;
        if accepted.record() != &decision
            || request::authorized(
                &self.request,
                &self.request_digest,
                &inputs,
                &accepted,
                previous,
            )? != record.request
        {
            return Err(
                PublicIdentityError::Corrupt("membership retained authority binding").into(),
            );
        }
        previous
            .clone()
            .extend_candidate(record)
            .map_err(PublicIdentityError::from)?;
        Ok(())
    }

    fn decision(&self, reader: &impl Read) -> StoreResult<AppliedAthleteIdentity> {
        let key = witness::receipt_key(&self.operation);
        let raw = reader
            .row(Rows::Receipt, &key)?
            .ok_or(PublicIdentityError::Corrupt("missing membership receipt"))?;
        let receipt: crate::Receipt = codec::decode(&key, &raw)?;
        let raw = reader
            .row(Rows::Decision, &self.decision_id)?
            .ok_or(PublicIdentityError::Corrupt("missing membership decision"))?;
        let decision: AppliedAthleteIdentity = codec::decode(&self.decision_id, &raw)?;
        if receipt.operation != key
            || receipt.digest != serialized_digest(self)?
            || receipt.appended != 1
            || receipt.credited
            || receipt.at != decision.observed_at
            || self.decision_id != decision.id
            || self.decision_digest != serialized_digest(&decision)?
        {
            return Err(PublicIdentityError::Corrupt("membership decision receipt binding").into());
        }
        Ok(decision)
    }
}

fn head<'de, D: serde::Deserializer<'de>>(parser: D) -> Result<RegistryHead, D::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Head {
        sequence: u64,
        digest: String,
    }
    let raw = Head::deserialize(parser)?;
    if !crate::journal::is_object_id(&raw.digest) {
        return Err(serde::de::Error::custom("invalid registry head digest"));
    }
    Ok(RegistryHead {
        sequence: raw.sequence,
        digest: raw.digest,
    })
}

fn empty_ids<'de, D: serde::Deserializer<'de>>(parser: D) -> Result<Vec<String>, D::Error> {
    <[String; 0]>::deserialize(parser)?;
    Ok(Vec::new())
}
