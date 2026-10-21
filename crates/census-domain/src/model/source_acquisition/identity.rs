use super::scalar::VersionOne;
use super::{AcquisitionDigest, AcquisitionWorkId, AttemptOrdinal, HopOrdinal};
use crate::model::StoreIdentity;
use serde::{Deserialize, Serialize};

mod slot;
pub(super) use slot::validate_meet_id;
pub use slot::AcquisitionSlotV1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionAttemptId {
    store_identity: StoreIdentity,
    work_id: AcquisitionWorkId,
    ordinal: AttemptOrdinal,
}
impl AcquisitionAttemptId {
    pub fn try_clone(&self) -> Result<Self, std::collections::TryReserveError> {
        Ok(Self {
            store_identity: self.store_identity.try_clone()?,
            work_id: self.work_id.try_clone()?,
            ordinal: self.ordinal,
        })
    }
    pub fn new(
        store_identity: StoreIdentity,
        work_id: AcquisitionWorkId,
        ordinal: AttemptOrdinal,
    ) -> Self {
        Self {
            store_identity,
            work_id,
            ordinal,
        }
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store_identity
    }
    pub fn work_id(&self) -> &AcquisitionWorkId {
        &self.work_id
    }
    pub fn ordinal(&self) -> AttemptOrdinal {
        self.ordinal
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionDispatchId {
    attempt: AcquisitionAttemptId,
    hop: HopOrdinal,
}
impl AcquisitionDispatchId {
    pub fn new(attempt: AcquisitionAttemptId, hop: HopOrdinal) -> Self {
        Self { attempt, hop }
    }
    pub fn attempt(&self) -> &AcquisitionAttemptId {
        &self.attempt
    }
    pub fn hop(&self) -> HopOrdinal {
        self.hop
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionOccurrenceRefV1 {
    version: VersionOne,
    attempt: AcquisitionAttemptId,
    terminal_digest: AcquisitionDigest,
}
impl AcquisitionOccurrenceRefV1 {
    pub fn try_clone(&self) -> Result<Self, std::collections::TryReserveError> {
        Ok(Self {
            version: self.version,
            attempt: self.attempt.try_clone()?,
            terminal_digest: self.terminal_digest.try_clone()?,
        })
    }
    pub fn new(attempt: AcquisitionAttemptId, terminal_digest: AcquisitionDigest) -> Self {
        Self {
            version: VersionOne,
            attempt,
            terminal_digest,
        }
    }
    pub fn attempt(&self) -> &AcquisitionAttemptId {
        &self.attempt
    }
    pub fn terminal_digest(&self) -> &AcquisitionDigest {
        &self.terminal_digest
    }
}
