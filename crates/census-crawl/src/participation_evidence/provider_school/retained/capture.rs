use super::metadata::{self, RetainedProviderSchoolCaptureMetadataV1};
use crate::participation_evidence::{bound, hash, invalid, RetainedHtmlCaptureV1};
#[cfg(test)]
use census_domain::evidence_subject_attachment::{AttachmentDigestV1, AttachmentObjectReferenceV1};
#[cfg(test)]
use census_store::EvidenceSubjectAttachmentStoreError;
use census_store::{PublicSchoolIntakeCaptureV1, RetainedCaptureRootV1, StoreError, StoreResult};
use chrono::{DateTime, Utc};
use serde::{Serialize, Serializer};

#[derive(Clone, Copy)]
pub enum RetainedProviderSchoolCaptureV1<'a> {
    Raw(&'a RetainedHtmlCaptureV1),
    Owned(&'a PublicSchoolIntakeCaptureV1),
}

impl RetainedProviderSchoolCaptureV1<'_> {
    pub(crate) fn body(&self) -> StoreResult<String> {
        match self {
            Self::Raw(capture) => capture.body(),
            Self::Owned(capture) => owned_body(capture),
        }
    }

    pub(crate) fn body_sha256(&self) -> &str {
        match self {
            Self::Raw(capture) => capture.body_sha256(),
            Self::Owned(capture) => capture.prepared().content_digest().as_str(),
        }
    }

    pub fn requested_url(&self) -> &str {
        match self {
            Self::Raw(capture) => capture.requested_url(),
            Self::Owned(capture) => capture.work().request().original_url(),
        }
    }

    #[cfg(test)]
    pub(crate) fn final_url(&self) -> &str {
        match self {
            Self::Raw(capture) => capture.final_url(),
            Self::Owned(capture) => capture.response().response_url(),
        }
    }

    #[cfg(test)]
    pub(crate) fn status(&self) -> u16 {
        match self {
            Self::Raw(capture) => capture.status(),
            Self::Owned(capture) => capture.response().status(),
        }
    }

    pub fn acquired_at(&self) -> StoreResult<DateTime<Utc>> {
        match self {
            Self::Raw(capture) => Ok(capture.acquired_at()),
            Self::Owned(capture) => DateTime::parse_from_rfc3339(capture.response().observed_at())
                .map(|instant| instant.with_timezone(&Utc))
                .map_err(|_| invalid("owned retained acquisition timestamp invalid")),
        }
    }

    #[cfg(test)]
    pub(crate) fn original_body_reference(&self) -> StoreResult<AttachmentObjectReferenceV1> {
        match self {
            Self::Raw(capture) => capture.original_body_reference(),
            Self::Owned(capture) => Ok(AttachmentObjectReferenceV1 {
                digest: AttachmentDigestV1::parse(capture.prepared().content_digest().as_str())
                    .map_err(EvidenceSubjectAttachmentStoreError::Domain)?,
                bytes: capture.prepared().content_bytes(),
            }),
        }
    }

    pub(crate) fn roster_metadata<'a>(
        &'a self,
        root: &'a RetainedCaptureRootV1,
        packet_bytes: &[u8],
        body: &str,
    ) -> StoreResult<RetainedProviderSchoolCaptureMetadataV1<'a>> {
        metadata::read(*self, root, packet_bytes, body)
    }
}

impl Serialize for RetainedProviderSchoolCaptureV1<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Raw(capture) => capture.serialize(serializer),
            Self::Owned(capture) => capture.serialize(serializer),
        }
    }
}

fn owned_body(capture: &PublicSchoolIntakeCaptureV1) -> StoreResult<String> {
    let bytes = capture.body();
    bound(bytes.len(), 8 * 1024 * 1024, "owned retained body bytes")?;
    if bytes.is_empty()
        || hash(bytes) != capture.prepared().content_digest().as_str()
        || u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)?
            != capture.prepared().content_bytes()
    {
        return Err(invalid("owned retained original body differs"));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| invalid("owned retained original body is not UTF-8"))?;
    let mut body = String::new();
    body.try_reserve_exact(text.len())
        .map_err(|_| invalid("owned retained original body allocation refused"))?;
    body.push_str(text);
    Ok(body)
}
