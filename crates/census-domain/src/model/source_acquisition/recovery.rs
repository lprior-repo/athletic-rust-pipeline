use super::{AcquisitionAttemptId, AcquisitionDigest, AcquisitionSlotId, CaptureDomainError};
use crate::model::StoreIdentity;
use serde::{Deserialize, Serialize};

mod assessment;
mod decision;
pub use assessment::{
    MissingBodyV1, MissingLocation, MissingMetadataV1, MissingPreparedAssessmentV1,
};
pub use decision::MissingPreparedDecisionV1;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TargetWire")]
pub struct PreparedRecoveryTarget {
    store_identity: StoreIdentity,
    slot_id: AcquisitionSlotId,
    attempt: AcquisitionAttemptId,
    prepared_digest: AcquisitionDigest,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetWire {
    store_identity: StoreIdentity,
    slot_id: AcquisitionSlotId,
    attempt: AcquisitionAttemptId,
    prepared_digest: AcquisitionDigest,
}
impl PreparedRecoveryTarget {
    pub fn try_clone(&self) -> Result<Self, std::collections::TryReserveError> {
        Ok(Self {
            store_identity: self.store_identity.try_clone()?,
            slot_id: self.slot_id.try_clone()?,
            attempt: self.attempt.try_clone()?,
            prepared_digest: self.prepared_digest.try_clone()?,
        })
    }
    pub fn new(
        store_identity: StoreIdentity,
        slot_id: AcquisitionSlotId,
        attempt: AcquisitionAttemptId,
        prepared_digest: AcquisitionDigest,
    ) -> Result<Self, CaptureDomainError> {
        if &store_identity != attempt.store_identity() {
            return Err(CaptureDomainError::Invalid {
                field: "recovery Store identity",
            });
        }
        Ok(Self {
            store_identity,
            slot_id,
            attempt,
            prepared_digest,
        })
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
}
impl TryFrom<TargetWire> for PreparedRecoveryTarget {
    type Error = CaptureDomainError;
    fn try_from(value: TargetWire) -> Result<Self, Self::Error> {
        Self::new(
            value.store_identity,
            value.slot_id,
            value.attempt,
            value.prepared_digest,
        )
    }
}
