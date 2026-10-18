use super::super::issuance::ledger::Ledger;
use super::*;

pub(in super::super) fn require_review(
    view: &StoreSnapshot<'_>,
    command: &ResultSchoolReviewCommandV1,
    materials: &[Material],
    ledger: &Ledger,
    attachments: &crate::CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    match &command.action {
        ResultSchoolActionV1::Accept { .. } => {
            require_current(view, &command.source, materials, attachments, grant)
        }
        ResultSchoolActionV1::Revoke { prior } => {
            grant
                .charge_steps(ledger.records.len())
                .map_err(grant_error)?;
            let original = ledger
                .active_records()
                .find(|record| {
                    record.command.operation == prior.operation
                        && record.record_digest == prior.digest
                })
                .ok_or(Error::SourceMismatch("revoke prior is not active"))?;
            if original.command.source != command.source {
                return Err(Error::SourceMismatch("revoke original source differs").into());
            }
            require_historical(
                view,
                &command.source,
                materials,
                attachments.retained_records(grant)?,
                grant,
            )
        }
    }
}
