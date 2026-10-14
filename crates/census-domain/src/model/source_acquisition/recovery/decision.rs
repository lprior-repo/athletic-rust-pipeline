use super::super::scalar::VersionOne;
use super::{
    AcquisitionAttemptId, AcquisitionDigest, AcquisitionSlotId, CaptureDomainError,
    MissingPreparedAssessmentV1, PreparedRecoveryTarget,
};
use crate::model::{serialized_digest, StoreIdentity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Reason {
    CloseWithoutSuppliedExactBody,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DecisionWire")]
pub struct MissingPreparedDecisionV1 {
    version: VersionOne,
    store_identity: StoreIdentity,
    slot_id: AcquisitionSlotId,
    attempt: AcquisitionAttemptId,
    prepared_digest: AcquisitionDigest,
    assessment: MissingPreparedAssessmentV1,
    reason: Reason,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DecisionWire {
    version: VersionOne,
    store_identity: StoreIdentity,
    slot_id: AcquisitionSlotId,
    attempt: AcquisitionAttemptId,
    prepared_digest: AcquisitionDigest,
    assessment: MissingPreparedAssessmentV1,
    reason: Reason,
}
impl MissingPreparedDecisionV1 {
    pub fn new(target: PreparedRecoveryTarget, assessment: MissingPreparedAssessmentV1) -> Self {
        Self {
            version: VersionOne,
            store_identity: target.store_identity,
            slot_id: target.slot_id,
            attempt: target.attempt,
            prepared_digest: target.prepared_digest,
            assessment,
            reason: Reason::CloseWithoutSuppliedExactBody,
        }
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store_identity
    }
    pub fn slot_id(&self) -> &AcquisitionSlotId {
        &self.slot_id
    }
    pub fn attempt(&self) -> &AcquisitionAttemptId {
        &self.attempt
    }
    pub fn prepared_digest(&self) -> &AcquisitionDigest {
        &self.prepared_digest
    }
    pub fn assessment(&self) -> &MissingPreparedAssessmentV1 {
        &self.assessment
    }
    pub fn digest(&self) -> Result<AcquisitionDigest, CaptureDomainError> {
        let digest = serialized_digest(&("prepared-missing-decision-v1", self))
            .map_err(|error| CaptureDomainError::Encoding(error.to_string()))?;
        AcquisitionDigest::parse(&digest)
    }
}
impl TryFrom<DecisionWire> for MissingPreparedDecisionV1 {
    type Error = CaptureDomainError;
    fn try_from(value: DecisionWire) -> Result<Self, Self::Error> {
        let (VersionOne, Reason::CloseWithoutSuppliedExactBody) = (value.version, value.reason);
        let target = PreparedRecoveryTarget::new(
            value.store_identity,
            value.slot_id,
            value.attempt,
            value.prepared_digest,
        )?;
        Ok(Self::new(target, value.assessment))
    }
}
