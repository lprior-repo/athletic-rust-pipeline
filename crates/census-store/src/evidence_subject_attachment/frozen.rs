use super::{material, EvidenceSubjectAttachmentStoreError as Error};
use crate::{StoreError, StoreResult};
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
    EvidenceSubjectAttachmentError,
};

mod archive;
mod bundle;
mod capability;
mod capture;
mod current;
mod roots;
pub(crate) mod source;

pub use bundle::FrozenEvidenceSubjectAttachmentBundleV1;
pub use capability::FrozenCurrentEvidenceSubjectAttachmentV1;
pub(super) use capture::capture;
pub use current::{
    replay_frozen_evidence_subject_attachment_references_v1,
    FrozenCurrentEvidenceSubjectAttachmentReferencesV1,
};
pub use source::FrozenRetainedCaptureSourceV1;

const MAX_ARCHIVE_BYTES: usize = 512 * 1024 * 1024;
const MAX_ROWS: usize = 40_000;
const MAX_PREFIXES: usize = MAX_ROWS + 2;
const SCHEMA: &str = "retained-evidence-subject-attachment-closure/v1";

fn steps(grant: &mut AttachmentReplayGrantV1, count: usize) -> StoreResult<()> {
    grant
        .charge_steps(count)
        .map_err(|source| Error::Domain(source).into())
}

fn bound(what: &'static str, needed: usize, available: usize) -> StoreResult<()> {
    if needed > available {
        return Err(StoreError::BudgetExceeded {
            what,
            needed: u64::try_from(needed).map_err(|_| StoreError::CounterOverflow)?,
            available: u64::try_from(available).map_err(|_| StoreError::CounterOverflow)?,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
