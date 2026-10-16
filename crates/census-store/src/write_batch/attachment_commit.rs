use super::StoreBatch;
use crate::evidence_subject_attachment::{AttachmentCommitGuard, AttachmentJournalPlanV1};
use crate::{Application, StoreError, StoreResult};

pub(crate) fn commit_evidence_subject_attachment(
    guard: AttachmentCommitGuard<'_>,
    plan: AttachmentJournalPlanV1<'_>,
) -> StoreResult<Application> {
    let store = guard.store();
    let parts = plan.into_parts(store)?;
    let mut prepared: StoreBatch<'_> = store.write_batch();
    prepared.journal = parts.rows;
    let outcome = prepared
        .commit_staged_at(
            Some((&parts.operation, &parts.digest)),
            store.db.batch(),
            Some(parts.committed_at),
        )
        .and_then(|application| {
            application.ok_or_else(|| StoreError::Invariant {
                detail: "attachment commit produced no receipt".into(),
            })
        });
    drop(guard);
    outcome
}
