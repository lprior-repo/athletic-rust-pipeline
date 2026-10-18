use super::*;
use crate::{Receipt, StoreError};
use chrono::Utc;
use fjall::PersistMode;

pub(super) fn apply(
    store: &Store,
    prepared: PreparedResultSchoolCorrespondenceV1<'_>,
    approved: &[u8],
    approval_digest: &ResultSchoolDigestV1,
) -> StoreResult<Application> {
    let PreparedResultSchoolCorrespondenceV1 {
        revision,
        command,
        approval,
        approval_digest: expected,
        state,
        ledger,
        attachment_fence,
        mut grant,
    } = prepared;
    guarded(&mut grant, |grant| {
        revision.ensure_origin(store, command.operation.as_str())?;
        grant.charge_bytes(approved.len()).map_err(grant_error)?;
        if approved != approval || approval_digest != &expected {
            return Err(Error::ApprovalMismatch.into());
        }
        let view = store.snapshot_locked();
        match state {
            PreparedState::Replay(expected_receipt) => {
                drop(ledger);
                let attachments =
                    view.read_current_evidence_subject_attachment_references_v1(grant)?;
                let ledger = super::ledger::from_view(&view, &attachments, grant)?;
                let receipt = super::replay::retained(&ledger, &command, &expected, grant)?;
                if super::super::codec::encode(&receipt, grant)?
                    != super::super::codec::encode(&expected_receipt, grant)?
                {
                    return Err(Error::OperationConflict.into());
                }
                Ok(Application::Repeated(receipt))
            }
            PreparedState::New(context) => {
                verify_live_guards(
                    store,
                    &view,
                    &revision,
                    &command,
                    &ledger,
                    &attachment_fence,
                    grant,
                )?;
                commit(store, command, ledger, context, grant)
            }
        }
    })
}

fn verify_live_guards(
    store: &Store,
    view: &crate::StoreSnapshot<'_>,
    revision: &StoreRevision<'_>,
    command: &ResultSchoolReviewCommandV1,
    ledger: &super::ledger::Ledger,
    attachment_fence: &crate::read::JournalNamespaceInventory<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    revision.ensure_current(store, command.operation.as_str())?;
    let (format, live_head) =
        view.with_roster_source_snapshot_v1(|snapshot, meta, _, _| -> StoreResult<_> {
            let format = super::super::read::snapshot_format(snapshot, meta, grant)?;
            let raw = super::super::read::value(
                snapshot,
                meta,
                super::super::wire::HEAD.as_bytes(),
                grant,
            )?;
            let head: super::super::wire::AuthorityHead = super::super::codec::decode(&raw, grant)?;
            Ok((format, head))
        })?;
    if format.lifecycle_revision != Some(super::super::LIFECYCLE)
        || format.schema_version != Some(2)
        || format.key_format != Some(1)
        || format.migration_target.is_some()
    {
        return Err(Error::EnrollmentRequired.into());
    }
    let (fresh_token, fresh_head) = super::super::enrollment::inspect::inputs(view, grant)?;
    if command.input_token != fresh_token || command.accepted_head != fresh_head {
        return Err(Error::InputMoved.into());
    }
    if command.evidence_generation != revision.evidence_generation()
        || command.derived_generation != revision.derived_generation()
    {
        return Err(Error::InputMoved.into());
    }
    if live_head.schema != "result-school-authority-head-v1"
        || live_head.inventory != ledger.inventory
        || live_head.inventory != command.expected_inventory
        || ledger.store != command.store
    {
        return Err(Error::InputMoved.into());
    }
    let current = crate::CurrentEvidenceSubjectAttachmentReferencesV1::current_physical_inventory(
        view, grant,
    )?;
    if attachment_fence.logical_roots() != current.logical_roots()
        || !attachment_fence
            .fingerprint()
            .same_contents(store, current.fingerprint())?
    {
        return Err(Error::InputMoved.into());
    }
    if store
        .receipt(&super::super::wire::issuance_operation(command))?
        .is_some()
    {
        return Err(Error::OperationConflict.into());
    }
    Ok(())
}

fn commit(
    store: &Store,
    command: ResultSchoolReviewCommandV1,
    mut ledger: super::ledger::Ledger,
    context: Vec<super::super::source::Material>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Application> {
    let evidence_generation = store.generations.next_entities()?;
    let checked = CheckedResultSchoolCommandV1::check(command, grant).map_err(Error::Domain)?;
    let committed_at = Utc::now();
    let record =
        prepare_result_school_record_v1(checked, committed_at, grant).map_err(Error::Domain)?;
    let operation = super::super::wire::issuance_operation(&record.command);
    let receipt = Receipt {
        operation,
        digest: record.record_digest.as_str().into(),
        at: committed_at.to_rfc3339(),
        appended: 0,
        credited: false,
    };
    let record_key = super::super::wire::record_key(ledger.records.len())?;
    ledger.inventory = next_result_school_inventory_v1(&ledger.inventory, &record, grant)
        .map_err(Error::Domain)?;
    let envelope = super::super::wire::RecordEnvelope {
        schema: "result-school-reviewed-entry-v1".into(),
        record,
        context,
    };
    let record_bytes = super::super::codec::bounded::encode(&envelope, grant)?;
    let receipt_bytes = super::super::codec::encode(&receipt, grant)?;
    ledger
        .records
        .try_reserve_exact(1)
        .map_err(|_| Error::BudgetExceeded)?;
    ledger
        .receipts
        .try_reserve_exact(1)
        .map_err(|_| Error::BudgetExceeded)?;
    ledger.records.push(envelope.record);
    ledger
        .receipts
        .push(super::super::codec::decode(&receipt_bytes, grant)?);
    ledger.projection(grant)?;
    let head = super::super::codec::encode(
        &super::super::wire::AuthorityHead {
            schema: "result-school-authority-head-v1".into(),
            inventory: ledger.inventory,
        },
        grant,
    )?;
    grant.charge_steps(3).map_err(grant_error)?;
    let mut batch = store.db.batch();
    batch.insert(&store.journal, record_key, record_bytes);
    batch.insert(&store.receipts, receipt.operation.as_bytes(), receipt_bytes);
    batch.insert(&store.meta, super::super::wire::HEAD, head);
    crate::generation::write_entities(&mut batch, &store.meta, evidence_generation);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    store.generations.publish_entities(evidence_generation);
    Ok(Application::Written(receipt))
}
