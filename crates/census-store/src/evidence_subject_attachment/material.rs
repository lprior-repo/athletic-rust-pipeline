use super::{EvidenceSubjectAttachmentStoreError as Error, RetainedCaptureLeafV1};
use crate::read::RetainedJournalReadSource;
use crate::{StoreError, StoreResult};
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentReplayGrantV1, EvidenceSubjectAttachmentError,
};
use census_domain::public_identity_history::ReplayLimits;
use serde::{Deserialize, Serialize};

mod inline;

pub enum AttachmentContextObjectV1<'a> {
    Inline {
        reference: &'a AttachmentObjectReferenceV1,
        bytes: &'a [u8],
    },
    Retained(&'a RetainedCaptureLeafV1),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "storage", deny_unknown_fields, rename_all = "snake_case")]
pub(crate) enum Material {
    Inline {
        reference: AttachmentObjectReferenceV1,
        data: String,
    },
    Retained {
        leaf: RetainedCaptureLeafV1,
    },
}

impl Material {
    pub(crate) fn reference(&self) -> &AttachmentObjectReferenceV1 {
        match self {
            Self::Inline { reference, .. } => reference,
            Self::Retained { leaf } => &leaf.object,
        }
    }

    pub(crate) fn introduce(
        input: &AttachmentContextObjectV1<'_>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        grant.ensure_available().map_err(Error::Domain)?;
        let result = match input {
            AttachmentContextObjectV1::Inline { reference, bytes } => {
                inline::encode(reference, bytes, grant)
            }
            AttachmentContextObjectV1::Retained(leaf) => {
                let encoded = super::encoding::object(*leaf, grant)?;
                grant.charge_steps(1).map_err(Error::Domain)?;
                let leaf = serde_json::from_slice(encoded.bytes())
                    .map_err(|source| Error::Domain(source.into()))?;
                Ok(Self::Retained { leaf })
            }
        };
        if result.is_err() {
            grant.poison();
        }
        result
    }

    pub(crate) fn verify(
        &self,
        view: &impl RetainedJournalReadSource,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<()> {
        match self {
            Self::Inline { reference, data } => inline::decode(reference, data, grant).map(|_| ()),
            Self::Retained { leaf } => super::external::verify_leaf(view, leaf, grant),
        }
    }

    pub(crate) fn inline_bytes(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<Vec<u8>> {
        match self {
            Self::Inline { reference, data } => inline::decode(reference, data, grant),
            Self::Retained { .. } => {
                Err(Error::Corrupt("target witness object must be inline").into())
            }
        }
    }
}

pub(super) fn allocate<T>(rows: &mut Vec<T>) -> StoreResult<()> {
    rows.try_reserve_exact(1)
        .map_err(|_| Error::Domain(EvidenceSubjectAttachmentError::Allocation).into())
}

pub(super) fn bounded_bytes(reference: &AttachmentObjectReferenceV1) -> StoreResult<usize> {
    let bytes = usize::try_from(reference.bytes).map_err(|_| StoreError::CounterOverflow)?;
    if bytes == 0 || bytes > ReplayLimits::SMALL.total_text_bytes {
        return Err(Error::Corrupt("context object byte bound").into());
    }
    Ok(bytes)
}
