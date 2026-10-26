use super::{invalid, MAX_PACKET_BYTES};
use crate::report::ReportResult;
use census_domain::evidence_subject_attachment::{AttachmentDigestV1, AttachmentObjectReferenceV1};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReportedProfileRootSchemaV1 {
    ProviderTargetPacketV1,
    OwnedPublicProfileCaptureV1,
    MilesplitProfilePrJournalV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReportedProfileRootV1 {
    pub(crate) schema: ReportedProfileRootSchemaV1,
    pub(crate) key: String,
    pub(crate) payload: AttachmentObjectReferenceV1,
}

impl ReportedProfileRootV1 {
    pub(super) fn retained(root: &census_store::RetainedCaptureRootV1) -> ReportResult<Self> {
        if root.schema != census_store::RetainedCaptureRootSchemaV1::ProviderTargetPacketV1 {
            return Err(invalid("reported profile root schema differs"));
        }
        Ok(Self {
            schema: ReportedProfileRootSchemaV1::ProviderTargetPacketV1,
            key: root.key.clone(),
            payload: root.payload.clone(),
        })
    }

    pub(crate) fn journal(digest: &str, bytes: u64) -> ReportResult<Self> {
        if bytes == 0 || bytes > MAX_PACKET_BYTES {
            return Err(invalid("profile journal root byte limit"));
        }
        let parsed = census_domain::evidence_subject_attachment::AttachmentDigestV1::parse(digest)
            .map_err(census_store::EvidenceSubjectAttachmentStoreError::Domain)
            .map_err(census_store::StoreError::from)?;
        Ok(Self {
            schema: ReportedProfileRootSchemaV1::MilesplitProfilePrJournalV1,
            key: digest.to_owned(),
            payload: AttachmentObjectReferenceV1 {
                digest: parsed,
                bytes,
            },
        })
    }

    pub(super) fn owned(bytes: &[u8]) -> ReportResult<Self> {
        let count = u64::try_from(bytes.len()).map_err(|_| invalid("profile root byte count"))?;
        if count == 0 || count > MAX_PACKET_BYTES {
            return Err(invalid("owned profile root byte limit"));
        }
        let packet = census_store::Store::decode_public_profile_capture_v1(bytes)?;
        let key = packet.reference()?.as_str().to_owned();
        let digest = AttachmentDigestV1::parse(&key)
            .map_err(census_store::EvidenceSubjectAttachmentStoreError::Domain)
            .map_err(census_store::StoreError::from)?;
        Ok(Self {
            schema: ReportedProfileRootSchemaV1::OwnedPublicProfileCaptureV1,
            key,
            payload: AttachmentObjectReferenceV1 {
                digest,
                bytes: count,
            },
        })
    }
}
