use super::scalar::{VersionOne, VersionTwo};
use crate::model::StoreIdentity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CaptureLifecycle {
    FreshQualification,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CaptureProtocol {
    SourceAcquisitionAttemptV1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CaptureConsumer {
    MilesplitStructuredWholeMeetV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureQualificationV1 {
    version: VersionOne,
    lifecycle: CaptureLifecycle,
    store_identity: StoreIdentity,
    protocol: CaptureProtocol,
    consumer: CaptureConsumer,
    cache_envelope_version: VersionTwo,
}
impl CaptureQualificationV1 {
    pub fn new(store_identity: StoreIdentity) -> Self {
        Self {
            version: VersionOne,
            lifecycle: CaptureLifecycle::FreshQualification,
            store_identity,
            protocol: CaptureProtocol::SourceAcquisitionAttemptV1,
            consumer: CaptureConsumer::MilesplitStructuredWholeMeetV1,
            cache_envelope_version: VersionTwo,
        }
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store_identity
    }
}
