use super::super::scalar::VersionOne;
use super::{AcquisitionDigest, CaptureDomainError, PreparedRecoveryTarget};
use crate::model::serialized_digest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingLocation {
    ArchiveDirectory,
    BodiesDirectory,
    CapturesDirectory,
    ContentCaptureDirectory,
    BodyLeaf,
    MetadataLeaf,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MissingBodyV1 {
    Missing { location: MissingLocation },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MissingMetadataV1 {
    Exact,
    Missing { location: MissingLocation },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AssessmentWire")]
pub struct MissingPreparedAssessmentV1 {
    version: VersionOne,
    body: MissingBodyV1,
    metadata: MissingMetadataV1,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssessmentWire {
    version: VersionOne,
    body: MissingBodyV1,
    metadata: MissingMetadataV1,
}
impl MissingPreparedAssessmentV1 {
    pub fn new(
        body: MissingLocation,
        metadata: MissingMetadataV1,
    ) -> Result<Self, CaptureDomainError> {
        if !matches!(
            body,
            MissingLocation::ArchiveDirectory
                | MissingLocation::BodiesDirectory
                | MissingLocation::BodyLeaf
        ) {
            return Err(CaptureDomainError::Invalid {
                field: "missing body location",
            });
        }
        if let MissingMetadataV1::Missing { location } = metadata {
            if !matches!(
                location,
                MissingLocation::ArchiveDirectory
                    | MissingLocation::CapturesDirectory
                    | MissingLocation::ContentCaptureDirectory
                    | MissingLocation::MetadataLeaf
            ) {
                return Err(CaptureDomainError::Invalid {
                    field: "missing metadata location",
                });
            }
        }
        let shared = matches!(
            metadata,
            MissingMetadataV1::Missing {
                location: MissingLocation::ArchiveDirectory
            }
        );
        if (body == MissingLocation::ArchiveDirectory) != shared {
            return Err(CaptureDomainError::Invalid {
                field: "shared missing archive parent",
            });
        }
        Ok(Self {
            version: VersionOne,
            body: MissingBodyV1::Missing { location: body },
            metadata,
        })
    }
    pub fn body(&self) -> MissingLocation {
        let MissingBodyV1::Missing { location } = self.body;
        location
    }
    pub fn metadata(&self) -> MissingMetadataV1 {
        self.metadata
    }
    pub fn digest(
        &self,
        target: &PreparedRecoveryTarget,
    ) -> Result<AcquisitionDigest, CaptureDomainError> {
        let digest = serialized_digest(&(
            "prepared-missing-assessment-v1",
            target.store_identity(),
            target.slot_id(),
            target.attempt(),
            target.prepared_digest(),
            self,
        ))
        .map_err(|error| CaptureDomainError::Encoding(error.to_string()))?;
        AcquisitionDigest::parse(&digest)
    }
}
impl TryFrom<AssessmentWire> for MissingPreparedAssessmentV1 {
    type Error = CaptureDomainError;
    fn try_from(value: AssessmentWire) -> Result<Self, Self::Error> {
        let VersionOne = value.version;
        let MissingBodyV1::Missing { location } = value.body;
        Self::new(location, value.metadata)
    }
}
