use super::scalar::{bounded, VersionOne};
use super::{
    AcquisitionDigest, AcquisitionSlotId, AcquisitionWorkId, AttemptOrdinal, CaptureDomainError,
    HopOrdinal,
};
use crate::model::{serialized_digest, StoreIdentity};
use crate::UsJurisdiction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Stage {
    Results,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Source {
    Milesplit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResourceRole {
    StructuredWholeMeet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SlotWire")]
pub struct AcquisitionSlotV1 {
    version: VersionOne,
    store_identity: StoreIdentity,
    owner_key: String,
    stage: Stage,
    historical_year: u16,
    source: Source,
    source_site: UsJurisdiction,
    source_meet_id: String,
    resource_role: ResourceRole,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SlotWire {
    version: VersionOne,
    store_identity: StoreIdentity,
    owner_key: String,
    stage: Stage,
    historical_year: u16,
    source: Source,
    source_site: UsJurisdiction,
    source_meet_id: String,
    resource_role: ResourceRole,
}
impl AcquisitionSlotV1 {
    pub fn new(
        store_identity: StoreIdentity,
        owner_key: &str,
        historical_year: u16,
        source_site: UsJurisdiction,
        source_meet_id: &str,
    ) -> Result<Self, CaptureDomainError> {
        bounded(owner_key, "owner key", 64)?;
        if owner_key.trim() != owner_key || !(2020..=2027).contains(&historical_year) {
            return Err(CaptureDomainError::Invalid {
                field: "slot scope",
            });
        }
        validate_meet_id(source_meet_id)?;
        Ok(Self {
            version: VersionOne,
            store_identity,
            owner_key: owner_key.to_string(),
            stage: Stage::Results,
            historical_year,
            source: Source::Milesplit,
            source_site,
            source_meet_id: source_meet_id.to_string(),
            resource_role: ResourceRole::StructuredWholeMeet,
        })
    }
    pub fn id(&self) -> Result<AcquisitionSlotId, CaptureDomainError> {
        let digest = serialized_digest(&("source-acquisition-slot-v1", self))
            .map_err(|e| CaptureDomainError::Encoding(e.to_string()))?;
        AcquisitionSlotId::parse(&digest)
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store_identity
    }
    pub fn owner_key(&self) -> &str {
        &self.owner_key
    }
    pub fn historical_year(&self) -> u16 {
        self.historical_year
    }
    pub fn source_site(&self) -> UsJurisdiction {
        self.source_site
    }
    pub fn source_meet_id(&self) -> &str {
        &self.source_meet_id
    }
}
impl TryFrom<SlotWire> for AcquisitionSlotV1 {
    type Error = CaptureDomainError;
    fn try_from(w: SlotWire) -> Result<Self, Self::Error> {
        let (VersionOne, Stage::Results, Source::Milesplit, ResourceRole::StructuredWholeMeet) =
            (w.version, w.stage, w.source, w.resource_role);
        Self::new(
            w.store_identity,
            &w.owner_key,
            w.historical_year,
            w.source_site,
            &w.source_meet_id,
        )
    }
}
pub(super) fn validate_meet_id(value: &str) -> Result<(), CaptureDomainError> {
    if value.len() > 20
        || value.is_empty()
        || value.starts_with('0')
        || !value.bytes().all(|b| b.is_ascii_digit())
        || value.parse::<u64>().is_err()
    {
        return Err(CaptureDomainError::Invalid {
            field: "source meet id",
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionAttemptId {
    store_identity: StoreIdentity,
    work_id: AcquisitionWorkId,
    ordinal: AttemptOrdinal,
}
impl AcquisitionAttemptId {
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
