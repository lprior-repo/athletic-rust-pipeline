use crate::{Store, StoreResult, StoreRevision};
use chrono::{DateTime, Utc};

pub(crate) struct AttachmentJournalPlanV1<'s> {
    revision: StoreRevision<'s>,
    rows: Vec<(Vec<u8>, Vec<u8>)>,
    operation: String,
    digest: String,
    committed_at: DateTime<Utc>,
}

pub(crate) struct AttachmentCommitPartsV1 {
    pub(crate) rows: Vec<(Vec<u8>, Vec<u8>)>,
    pub(crate) operation: String,
    pub(crate) digest: String,
    pub(crate) committed_at: DateTime<Utc>,
}

impl<'s> AttachmentJournalPlanV1<'s> {
    pub(super) fn new(
        revision: StoreRevision<'s>,
        rows: Vec<(Vec<u8>, Vec<u8>)>,
        operation: String,
        digest: String,
        committed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            revision,
            rows,
            operation,
            digest,
            committed_at,
        }
    }

    pub(crate) fn into_parts(self, store: &Store) -> StoreResult<AttachmentCommitPartsV1> {
        self.revision
            .ensure_origin(store, "commit evidence subject attachment")?;
        Ok(AttachmentCommitPartsV1 {
            rows: self.rows,
            operation: self.operation,
            digest: self.digest,
            committed_at: self.committed_at,
        })
    }
}
