use super::*;

pub(super) fn prepare<'s>(
    store: &'s Store,
    command: ResultCalendarReviewCommandV1,
    inspection: ResultCalendarCorrespondenceInspectionV1<'s>,
    context: &[AttachmentContextObjectV1<'_>],
) -> StoreResult<PreparedResultCalendarCorrespondenceV1<'s>> {
    let ResultCalendarCorrespondenceInspectionV1 {
        revision,
        store: identity,
        inventory,
        input_token,
        public_head,
        mut grant,
    } = inspection;
    let (command, approval, approval_digest, materials, ledger, attachment_fence) =
        guarded(&mut grant, |grant| {
            revision.ensure_origin(store, command.operation.as_str())?;
            if command.store != identity
                || command.expected_inventory != inventory
                || command.input_token != input_token
                || command.accepted_head != public_head
                || command.evidence_generation != revision.evidence_generation()
                || command.derived_generation != revision.derived_generation()
            {
                return Err(Error::InputMoved.into());
            }
            let view = store.snapshot_locked();
            let (ledger, attachments) = require_current(store, &view, &revision, &command, grant)?;
            let materials = super::super::source::introduce(&command.source, context, grant)?;
            super::super::source::validate_inventory(&command.source, &materials, grant)?;
            require_source(&view, &command, &materials, &ledger, &attachments, grant)?;
            let (ledger, command, approval, digest, attachment_fence) =
                seal_preparation(ledger, command, attachments, grant)?;
            Ok((
                command,
                approval,
                digest,
                materials,
                ledger,
                attachment_fence,
            ))
        })?;
    Ok(PreparedResultCalendarCorrespondenceV1 {
        revision,
        command,
        approval,
        approval_digest,
        state: PreparedState::New(materials),
        ledger,
        attachment_fence,
        grant,
    })
}

fn require_source(
    view: &crate::StoreSnapshot<'_>,
    command: &ResultCalendarReviewCommandV1,
    materials: &[super::super::source::Material],
    ledger: &super::ledger::Ledger,
    attachments: &crate::CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    match &command.action {
        ResultCalendarActionV1::Accept { .. } => super::super::source::require_current(
            view,
            &command.source,
            materials,
            attachments,
            grant,
        ),
        ResultCalendarActionV1::Revoke { prior } => {
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
            let originals = attachments.reviewed_source_v1(grant)?;
            let source = super::super::source::checked::Source::new(view, &ledger.checked_roots);
            super::super::source::require_historical(
                &source,
                &command.source,
                materials,
                &originals,
                grant,
            )
        }
    }
}

fn seal_preparation<'v, 's>(
    ledger: super::ledger::Ledger,
    command: ResultCalendarReviewCommandV1,
    attachments: crate::CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(
    super::ledger::Ledger,
    ResultCalendarReviewCommandV1,
    Vec<u8>,
    ResultCalendarDigestV1,
    crate::read::JournalNamespaceInventory<'s>,
)> {
    let checked = CheckedResultCalendarCommandV1::check(command, grant).map_err(Error::Domain)?;
    let approval = super::super::wire::review_approval(&checked, grant)?;
    let digest = super::super::codec::digest(&approval, grant)?;
    let reviewed_at = checked.command(grant).map_err(Error::Domain)?.reviewed_at;
    let (ledger, record) = validate_proposed(ledger, checked, reviewed_at, grant)?;
    let command = record.command;
    let attachment_fence = attachments.into_physical_inventory(grant)?;
    Ok((ledger, command, approval, digest, attachment_fence))
}

fn validate_proposed(
    mut ledger: super::ledger::Ledger,
    checked: CheckedResultCalendarCommandV1,
    reviewed_at: chrono::DateTime<chrono::Utc>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(super::ledger::Ledger, ResultCalendarReviewedRecordV1)> {
    let record =
        prepare_result_calendar_record_v1(checked, reviewed_at, grant).map_err(Error::Domain)?;
    let next_inventory = next_result_calendar_inventory_v1(&ledger.inventory, &record, grant)
        .map_err(Error::Domain)?;
    let previous_inventory = std::mem::replace(&mut ledger.inventory, next_inventory);
    grant
        .charge_bytes(std::mem::size_of::<ResultCalendarReviewedRecordV1>())
        .map_err(grant_error)?;
    ledger
        .records
        .try_reserve_exact(1)
        .map_err(|_| Error::BudgetExceeded)?;
    ledger
        .receipts
        .try_reserve_exact(1)
        .map_err(|_| Error::BudgetExceeded)?;
    ledger.receipts.push(crate::Receipt {
        operation: super::super::wire::issuance_operation(&record.command),
        digest: record.record_digest.as_str().into(),
        at: record.committed_at.to_rfc3339(),
        appended: 0,
        credited: false,
    });
    ledger.records.push(record);
    ledger.projection(grant)?;
    let record = ledger
        .records
        .pop()
        .ok_or(Error::Corrupt("validate_proposed record"))?;
    ledger
        .receipts
        .pop()
        .ok_or(Error::Corrupt("validate_proposed receipt"))?;
    ledger.inventory = previous_inventory;
    Ok((ledger, record))
}

pub(super) fn prepare_replay(
    store: &Store,
    command: ResultCalendarReviewCommandV1,
    mut grant: AttachmentReplayGrantV1,
) -> StoreResult<PreparedResultCalendarCorrespondenceV1<'_>> {
    let (command, approval, approval_digest, receipt, ledger, attachment_fence) =
        guarded(&mut grant, |grant| {
            let view = store.snapshot_locked();
            let attachments = view.read_current_evidence_subject_attachment_references_v1(grant)?;
            let ledger = super::ledger::from_view(&view, &attachments, grant)?;
            let checked =
                CheckedResultCalendarCommandV1::check(command, grant).map_err(Error::Domain)?;
            let approval = super::super::wire::review_approval(&checked, grant)?;
            let approval_digest = super::super::codec::digest(&approval, grant)?;
            let command = checked.into_command(grant).map_err(Error::Domain)?;
            let receipt = super::replay::retained(&ledger, &command, &approval_digest, grant)?;
            let attachment_fence = attachments.into_physical_inventory(grant)?;
            Ok((
                command,
                approval,
                approval_digest,
                receipt,
                ledger,
                attachment_fence,
            ))
        })?;
    Ok(PreparedResultCalendarCorrespondenceV1 {
        revision: store.snapshot_locked().revision(),
        command,
        approval,
        approval_digest,
        state: PreparedState::Replay(receipt),
        ledger,
        attachment_fence,
        grant,
    })
}

pub(super) fn require_current<'v, 's>(
    store: &Store,
    view: &'v crate::StoreSnapshot<'s>,
    revision: &StoreRevision<'_>,
    command: &ResultCalendarReviewCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(
    super::ledger::Ledger,
    crate::CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's>,
)> {
    super::super::wire::validate_review_command(command)?;
    revision.ensure_current(store, command.operation.as_str())?;
    let attachments = view.read_current_evidence_subject_attachment_references_v1(grant)?;
    let ledger = super::ledger::from_view(view, &attachments, grant)?;
    let (token, head) = super::super::enrollment::inspect::inputs(view, grant)?;
    if ledger.store != command.store
        || command.expected_inventory != ledger.inventory
        || command.input_token != token
        || command.accepted_head != head
        || command.evidence_generation != revision.evidence_generation()
        || command.derived_generation != revision.derived_generation()
    {
        return Err(Error::InputMoved.into());
    }
    if store
        .receipt(&super::super::wire::issuance_operation(command))?
        .is_some()
    {
        return Err(Error::OperationConflict.into());
    }
    Ok((ledger, attachments))
}
