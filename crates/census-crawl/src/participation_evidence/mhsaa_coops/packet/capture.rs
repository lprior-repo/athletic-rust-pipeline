use super::{body, provenance, refused};
use crate::participation_evidence::RetainedHttpProvenanceV1;
use census_domain::evidence_subject_attachment::{AttachmentDigestV1, AttachmentObjectReferenceV1};
use census_store::{EvidenceSubjectAttachmentStoreError, StoreError, StoreResult};
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct MhsaaCoopPdfCaptureV1 {
    pub(super) requested_url: String,
    pub(super) final_url: String,
    pub(super) status: u16,
    pub(super) acquired_at: DateTime<Utc>,
    pub(super) body_bytes: usize,
    pub(super) body_sha256: String,
    pub(super) body_base64: String,
    pub(super) provenance: RetainedHttpProvenanceV1,
    #[serde(skip)]
    pub(super) original: Vec<u8>,
}

impl MhsaaCoopPdfCaptureV1 {
    pub fn new(
        requested: &str,
        final_url: &str,
        status: u16,
        acquired_at: DateTime<Utc>,
        original: &[u8],
        provenance: RetainedHttpProvenanceV1,
    ) -> StoreResult<Self> {
        provenance::validate(
            requested,
            final_url,
            status,
            acquired_at,
            original.len(),
            &provenance,
        )?;
        let original = body::copy(original)?;
        Ok(Self {
            requested_url: requested.into(),
            final_url: final_url.into(),
            status,
            acquired_at,
            body_bytes: original.len(),
            body_sha256: super::super::hash(&original),
            body_base64: body::encode(&original)?,
            provenance,
            original,
        })
    }

    pub fn requested_url(&self) -> &str {
        &self.requested_url
    }
    pub fn final_url(&self) -> &str {
        &self.final_url
    }
    pub fn status(&self) -> u16 {
        self.status
    }
    pub fn acquired_at(&self) -> DateTime<Utc> {
        self.acquired_at
    }
    pub fn body_sha256(&self) -> &str {
        &self.body_sha256
    }
    pub fn original_bytes(&self) -> &[u8] {
        &self.original
    }
    pub fn provenance(&self) -> &RetainedHttpProvenanceV1 {
        &self.provenance
    }

    pub fn original_body_reference(&self) -> StoreResult<AttachmentObjectReferenceV1> {
        self.validate()?;
        let digest = AttachmentDigestV1::parse(&self.body_sha256)
            .map_err(EvidenceSubjectAttachmentStoreError::Domain)
            .map_err(StoreError::from)?;
        let bytes = u64::try_from(self.body_bytes).map_err(|_| StoreError::CounterOverflow)?;
        Ok(AttachmentObjectReferenceV1 { digest, bytes })
    }

    pub(super) fn validate(&self) -> StoreResult<()> {
        provenance::validate(
            &self.requested_url,
            &self.final_url,
            self.status,
            self.acquired_at,
            self.body_bytes,
            &self.provenance,
        )?;
        if self.original.len() != self.body_bytes
            || super::super::hash(&self.original) != self.body_sha256
            || body::encode(&self.original)? != self.body_base64
        {
            return Err(refused(
                "MHSAA co-op original PDF length, hash or encoding differs",
            ));
        }
        Ok(())
    }
}
