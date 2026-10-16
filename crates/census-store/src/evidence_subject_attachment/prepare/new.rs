use super::*;
use crate::StoreSnapshot;

pub(super) struct NewPreparation<'s, 'a> {
    pub(super) command: RetainedEvidenceSubjectAttachmentCommandV1,
    pub(super) inspection: EvidenceSubjectAttachmentInspectionV1<'s>,
    pub(super) supplied: &'a [AttachmentContextObjectV1<'a>],
    pub(super) approval: Vec<u8>,
}

pub(super) fn prepare<'s>(
    store: &'s Store,
    view: &StoreSnapshot<'s>,
    input: NewPreparation<'s, '_>,
    mut ledger: Ledger<'s>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<PreparedParts<'s>> {
    let NewPreparation {
        command,
        inspection,
        supplied,
        approval,
    } = input;
    require_fences(store, view, &command, &inspection, &ledger, grant)?;
    let introduced = context::introductions(view, &inspection, supplied, &ledger, grant)?;
    let prepared_digest = command
        .prepared_digest_with_grant(grant)
        .map_err(Error::Domain)?;
    let approval_digest = command
        .approval_digest_with_grant(grant)
        .map_err(Error::Domain)?;
    let committed_at = command.reviewed_at;
    let mut record = ReviewedReferenceRecordV1 {
        command,
        prepared_digest,
        approval_digest,
        committed_at,
        record_digest: AttachmentDigestV1::parse(&"0".repeat(64)).map_err(Error::Domain)?,
    };
    record.record_digest = record
        .calculated_record_digest_with_grant(grant)
        .map_err(Error::Domain)?;
    let inputs = super::super::ledger::wire::EntryInputs {
        capture: inspection.inputs.capture.reference().clone(),
        token: inspection.inputs.token.reference().clone(),
    };
    ledger.append_proposed(view, record, introduced, inputs, grant)?;
    ledger.validate_projection(&inspection.store, grant)?;
    let record = ledger
        .records
        .last()
        .ok_or(Error::Corrupt("prepared record missing"))?;
    super::super::target::verify_historical(&ledger, record, grant)?;
    let record = ledger
        .records
        .pop()
        .ok_or(Error::Corrupt("prepared record missing"))?;
    let introduced = ledger
        .introductions
        .pop()
        .ok_or(Error::Corrupt("prepared context missing"))?;
    Ok((
        view.revision(),
        record,
        approval,
        PreparedState::New {
            inspection,
            introduced,
        },
    ))
}

fn require_fences(
    store: &Store,
    view: &StoreSnapshot<'_>,
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    ledger: &Ledger<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    inspection
        .revision
        .ensure_current(store, command.operation.as_str())?;
    if command.store != inspection.store
        || command.expected_public_head != inspection.head
        || command.expected_evidence_generation != inspection.evidence_generation()
        || command.expected_derived_generation != inspection.derived_generation()
        || command.action != inspection.action
        || command.target != inspection.target
        || command.reviewed_context.accepted_target_witness != inspection.witness
    {
        return Err(Error::InputMoved.into());
    }
    if command.expected_attachment_inventory != inspection.ledger.inventory
        || command.expected_attachment_inventory != ledger.inventory
        || !inspection
            .ledger
            .physical
            .fingerprint()
            .same_contents(store, ledger.physical.fingerprint())?
    {
        return Err(Error::InventoryMoved.into());
    }
    super::super::inputs::require_head(view, &inspection.head, grant)?;
    inspection.inputs.require_current(
        store,
        view,
        command.expected_evidence_generation,
        command.expected_derived_generation,
        grant,
    )?;
    for reference in inspection.issued_objects() {
        context::require_declared(command, reference, grant)?;
    }
    Ok(())
}
