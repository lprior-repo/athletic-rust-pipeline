use super::{codec, IdentityCommitOutcomeV1, PublicIdentityError, OPERATION_PREFIX};
use crate::{StoreError, StoreResult};
use census_domain::model::{
    serialized_digest, AppliedAthleteIdentity, AthleteCandidateId, StoreIdentity,
};
use census_domain::public_identity_history::{
    HistoryRecord, HistoryRequest, PublicAthleteId, RegistryHead,
};
use fjall::{Keyspace, Readable, Snapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Witness {
    pub version: u32,
    pub operation: String,
    pub candidate: String,
    pub request_digest: String,
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub snapshot_sequence: u64,
    pub record_digest: String,
    pub decision_id: String,
    pub decision_digest: String,
    pub public_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_context: Option<String>,
}

pub(super) fn request_digest(
    identity: &StoreIdentity,
    operation: &str,
    candidate: &AthleteCandidateId,
) -> StoreResult<String> {
    if operation.is_empty() || operation.len() > 256 || operation.chars().any(char::is_control) {
        return Err(PublicIdentityError::OperationConflict.into());
    }
    Ok(serialized_digest(&(
        "public-identity-establish-request-v1",
        identity,
        operation,
        candidate,
    ))?)
}

pub(super) fn key(sequence: u64) -> String {
    format!("{OPERATION_PREFIX}{sequence:020}")
}

pub(super) fn receipt_key(operation: &str) -> String {
    format!("{OPERATION_PREFIX}{operation}")
}

impl Witness {
    #[cfg(test)]
    pub(super) fn version(&self) -> u32 {
        self.version
    }

    pub(super) fn require_qualification_version(
        &self,
        version: census_domain::public_identity_qualification::PublicIdentityQualificationVersion,
    ) -> StoreResult<()> {
        if self.version != version.witness_version() {
            return Err(
                PublicIdentityError::Corrupt("witness qualification version mismatch").into(),
            );
        }
        match (self.version, self.authority_context.as_deref()) {
            (1, None) => Ok(()),
            (2, Some(root)) if crate::journal::is_object_id(root) => Ok(()),
            _ => Err(PublicIdentityError::Corrupt("authority context version binding").into()),
        }
    }

    pub(super) fn new(
        operation: &str,
        candidate: &AthleteCandidateId,
        digest: String,
        snapshot: &crate::StoreSnapshot<'_>,
        prepared: &census_domain::public_identity_history::PreparedHistoryRecord,
        decision: &AppliedAthleteIdentity,
        authority_context: &str,
    ) -> StoreResult<Self> {
        let [public_id] = prepared.established_ids.as_slice() else {
            return Err(PublicIdentityError::Corrupt("establishment cardinality").into());
        };
        Ok(Self {
            version: 2,
            operation: operation.to_owned(),
            candidate: candidate.to_string(),
            request_digest: digest,
            evidence_generation: snapshot.evidence_generation(),
            derived_generation: snapshot.derived_generation(),
            snapshot_sequence: snapshot.sequence(),
            record_digest: prepared.uncommitted_record.digest.clone(),
            decision_id: decision.id.clone(),
            decision_digest: serialized_digest(decision)?,
            public_id: public_id.as_str().to_owned(),
            authority_context: Some(authority_context.to_owned()),
        })
    }

    pub fn outcome(
        &self,
        record: &HistoryRecord,
        replayed: bool,
    ) -> StoreResult<IdentityCommitOutcomeV1> {
        Ok(IdentityCommitOutcomeV1 {
            public_id: PublicAthleteId::parse(&self.public_id)
                .map_err(PublicIdentityError::from)?,
            head: RegistryHead {
                sequence: record.sequence,
                digest: record.digest.clone(),
            },
            replayed,
        })
    }
    pub fn validate(&self, record: &HistoryRecord, public_id: &PublicAthleteId) -> StoreResult<()> {
        let HistoryRequest::Establish { binding, evidence } = &record.request else {
            return Err(PublicIdentityError::Corrupt("unsupported acceptance operation").into());
        };
        if !matches!(self.version, 1 | 2)
            || self.candidate != binding.candidate.as_str()
            || self.record_digest != record.digest
            || self.decision_id != evidence.application_id
            || self.public_id != public_id.as_str()
            || self.request_digest
                != request_digest(&record.installation, &self.operation, &binding.candidate)?
        {
            return Err(PublicIdentityError::Corrupt("commit witness binding").into());
        }
        Ok(())
    }
}

pub(super) fn verify_rows(
    reader: &impl super::retained::Read,
    identity: &StoreIdentity,
    witness: &Witness,
    evidence: &census_domain::public_identity_history::CandidateDecisionReference,
    binding: &census_domain::public_identity_history::BindingClaim,
) -> StoreResult<()> {
    let key = receipt_key(&witness.operation);
    let raw = reader
        .row(super::retained::Rows::Receipt, &key)?
        .ok_or(PublicIdentityError::Corrupt("missing commit receipt"))?;
    let receipt: crate::Receipt = codec::decode(&key, &raw)?;
    if receipt.operation != key
        || receipt.digest != serialized_digest(witness)?
        || receipt.appended != 1
        || receipt.credited
    {
        return Err(PublicIdentityError::Corrupt("commit receipt binding").into());
    }
    let raw = reader
        .row(super::retained::Rows::Decision, &witness.decision_id)?
        .ok_or(PublicIdentityError::Corrupt("missing accepted decision"))?;
    let decision: AppliedAthleteIdentity = codec::decode(&witness.decision_id, &raw)?;
    if receipt.at != decision.observed_at
        || decision.id != witness.decision_id
        || serialized_digest(&decision)? != witness.decision_digest
        || !evidence
            .matches_applied(
                &decision,
                census_domain::public_identity_history::ReplayLimits::SMALL,
            )
            .map_err(PublicIdentityError::from)?
    {
        return Err(PublicIdentityError::Corrupt("accepted decision digest").into());
    }
    if let Some(root) = &witness.authority_context {
        let inputs = reader.context(root, identity)?;
        let source = super::context::verify_source_inputs(&inputs, &decision)?;
        let expected =
            super::commit::source_binding(&binding.candidate, source, &witness.request_digest);
        if binding != &expected {
            return Err(PublicIdentityError::Corrupt("retained authority source binding").into());
        }
    }
    Ok(())
}

pub(super) fn verify_count(
    snapshot: &Snapshot,
    keyspace: &Keyspace,
    expected: usize,
) -> StoreResult<()> {
    let count = snapshot
        .prefix(keyspace, "public_identity")
        .take(expected.saturating_add(1))
        .try_fold(0usize, |count, guard| {
            guard.key().map_err(|source| StoreError::Read { source })?;
            count.checked_add(1).ok_or(StoreError::CounterOverflow)
        })?;
    if count != expected {
        return Err(PublicIdentityError::Corrupt("commit namespace cardinality").into());
    }
    Ok(())
}
