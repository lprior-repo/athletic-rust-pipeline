use super::qualification::{CaptureConsumer, CaptureLifecycle};
use super::scalar::VersionTwo;
use super::{CaptureProtocolVersion, CaptureQualificationV1};
use crate::model::StoreIdentity;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Protocol {
    SourceAcquisitionAttemptV2,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureQualificationV2 {
    version: VersionTwo,
    lifecycle: CaptureLifecycle,
    store_identity: StoreIdentity,
    protocol: Protocol,
    consumer: CaptureConsumer,
    cache_envelope_version: VersionTwo,
}
impl CaptureQualificationV2 {
    pub fn new(store_identity: StoreIdentity) -> Self {
        Self {
            version: VersionTwo,
            lifecycle: CaptureLifecycle::FreshQualification,
            store_identity,
            protocol: Protocol::SourceAcquisitionAttemptV2,
            consumer: CaptureConsumer::MilesplitStructuredWholeMeetV1,
            cache_envelope_version: VersionTwo,
        }
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store_identity
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureQualification {
    V1(CaptureQualificationV1),
    V2(CaptureQualificationV2),
}
impl CaptureQualification {
    pub fn store_identity(&self) -> &StoreIdentity {
        match self {
            Self::V1(record) => record.store_identity(),
            Self::V2(record) => record.store_identity(),
        }
    }
    pub fn protocol(&self) -> CaptureProtocolVersion {
        match self {
            Self::V1(_) => CaptureProtocolVersion::V1,
            Self::V2(_) => CaptureProtocolVersion::V2,
        }
    }
}
impl Serialize for CaptureQualification {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::V1(record) => record.serialize(serializer),
            Self::V2(record) => record.serialize(serializer),
        }
    }
}
