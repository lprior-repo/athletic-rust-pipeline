use super::*;

pub(super) fn prepare<'s>(
    store: &'s Store,
    command: ResultSchoolReviewCommandV1,
    inspection: ResultSchoolCorrespondenceInspectionV1<'s>,
    context: &[AttachmentContextObjectV1<'_>],
) -> StoreResult<PreparedResultSchoolCorrespondenceV1<'s>> {
    let ResultSchoolCorrespondenceInspectionV1 {
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
            super::super::source::require_review(
                &view,
                &command,
                &materials,
                &ledger,
                &attachments,
                grant,
            )?;
            let (command, approval, digest, ledger, attachment_fence) =
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
    Ok(PreparedResultSchoolCorrespondenceV1 {
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

fn seal_preparation<'v, 's>(
    ledger: super::ledger::Ledger,
    command: ResultSchoolReviewCommandV1,
    attachments: crate::CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(
    ResultSchoolReviewCommandV1,
    Vec<u8>,
    ResultSchoolDigestV1,
    super::ledger::Ledger,
    crate::read::JournalNamespaceInventory<'s>,
)> {
    let (ledger, command) = validate_proposed(ledger, command, grant)?;
    let attachment_fence = attachments.into_physical_inventory(grant)?;
    let approval = super::super::wire::review_approval(&command, grant)?;
    let digest = super::super::codec::digest(&approval, grant)?;
    Ok((command, approval, digest, ledger, attachment_fence))
}

pub(super) fn prepare_replay(
    store: &Store,
    command: ResultSchoolReviewCommandV1,
    mut grant: AttachmentReplayGrantV1,
) -> StoreResult<PreparedResultSchoolCorrespondenceV1<'_>> {
    let (approval, approval_digest, receipt, ledger, attachment_fence) =
        guarded(&mut grant, |grant| {
            let view = store.snapshot_locked();
            let attachments = view.read_current_evidence_subject_attachment_references_v1(grant)?;
            let ledger = super::ledger::from_view(&view, &attachments, grant)?;
            let approval = super::super::wire::review_approval(&command, grant)?;
            let digest = super::super::codec::digest(&approval, grant)?;
            let receipt = super::replay::retained(&ledger, &command, &digest, grant)?;
            let attachment_fence = attachments.into_physical_inventory(grant)?;
            Ok((approval, digest, receipt, ledger, attachment_fence))
        })?;
    Ok(PreparedResultSchoolCorrespondenceV1 {
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
    command: &ResultSchoolReviewCommandV1,
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

pub(super) fn validate_proposed(
    mut ledger: super::ledger::Ledger,
    command: ResultSchoolReviewCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(super::ledger::Ledger, ResultSchoolReviewCommandV1)> {
    let reviewed_at = command.reviewed_at;
    let checked = CheckedResultSchoolCommandV1::check(command, grant).map_err(Error::Domain)?;
    let record =
        prepare_result_school_record_v1(checked, reviewed_at, grant).map_err(Error::Domain)?;
    let next_inventory = next_result_school_inventory_v1(&ledger.inventory, &record, grant)
        .map_err(Error::Domain)?;
    let previous_inventory = std::mem::replace(&mut ledger.inventory, next_inventory);
    grant
        .charge_bytes(std::mem::size_of::<ResultSchoolReviewedRecordV1>())
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
    Ok((ledger, record.command))
}
