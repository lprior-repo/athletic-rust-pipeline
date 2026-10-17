use super::*;
use crate::StoreSnapshot;
use census_domain::evidence_subject_attachment::CheckedAttachmentCommandV1;

pub(super) struct NewPreparation<'s, 'a> {
    pub(super) command: CheckedAttachmentCommandV1,
    pub(super) inspection: EvidenceSubjectAttachmentInspectionV1<'s>,
    pub(super) supplied: &'a [AttachmentContextObjectV1<'a>],
    pub(super) approval: Vec<u8>,
}

pub(super) fn prepare<'s>(
    store: &'s Store,
    view: &StoreSnapshot<'s>,
    input: NewPreparation<'s, '_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<PreparedParts<'s>> {
    let NewPreparation {
        command,
        mut inspection,
        supplied,
        approval,
    } = input;
    require_fences(store, view, &command, &inspection, grant)?;
    let introduced =
        context::introductions(view, &inspection, supplied, &inspection.state.ledger, grant)?;
    let record = command
        .into_draft_with_grant(grant)
        .map_err(Error::Domain)?;
    let inputs = super::super::ledger::wire::EntryInputs {
        capture: inspection.state.inputs.capture.reference().clone(),
        token: inspection.state.inputs.token.reference().clone(),
    };
    let state = &mut *inspection.state;
    let ledger = &mut state.ledger;
    let previous = ledger.append_proposed(view, record, introduced, inputs, grant)?;
    ledger.validate_projection(&state.store, grant)?;
    let record = ledger
        .records
        .last()
        .ok_or(Error::Corrupt("prepared record missing"))?;
    super::super::target::verify_historical(ledger, record, grant)?;
    let (record, introduced) = restore(ledger, previous, &state.inputs, grant)?;
    Ok((
        view.revision(),
        approval,
        PreparedState::New(Box::new(NewPrepared {
            record,
            inspection,
            introduced,
        })),
    ))
}

fn restore(
    ledger: &mut Ledger<'_>,
    previous: census_domain::evidence_subject_attachment::AttachmentInventoryV1,
    expected: &super::super::inputs::Inputs,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(CheckedReviewedReferenceRecordV1, Vec<Material>)> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    let expected_len = ledger
        .receipts
        .len()
        .checked_add(1)
        .ok_or(crate::StoreError::CounterOverflow)?;
    if ledger.records.len() != expected_len
        || ledger.introductions.len() != expected_len
        || ledger.input_refs.len() != expected_len
    {
        return Err(Error::Corrupt("prepared ledger restoration").into());
    }
    let record = ledger
        .records
        .pop_with_grant(grant)
        .map_err(Error::Domain)?;
    let introduced = ledger
        .introductions
        .pop()
        .ok_or(Error::Corrupt("prepared context missing"))?;
    let inputs = ledger
        .input_refs
        .pop()
        .ok_or(Error::Corrupt("prepared inputs missing"))?;
    if inputs.capture != *expected.capture.reference()
        || inputs.token != *expected.token.reference()
    {
        return Err(Error::Corrupt("prepared ledger restoration").into());
    }
    ledger.inventory = previous;
    Ok((record, introduced))
}

fn require_fences(
    store: &Store,
    view: &StoreSnapshot<'_>,
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    inspection
        .state
        .revision
        .ensure_current(store, command.operation.as_str())?;
    if command.store != inspection.state.store
        || command.expected_public_head != inspection.state.head
        || command.expected_evidence_generation != inspection.evidence_generation()
        || command.expected_derived_generation != inspection.derived_generation()
        || command.action != inspection.state.action
        || command.target != inspection.state.target
        || command.reviewed_context.accepted_target_witness != inspection.state.witness
    {
        return Err(Error::InputMoved.into());
    }
    if command.expected_attachment_inventory != inspection.state.ledger.inventory {
        return Err(Error::InventoryMoved.into());
    }
    super::super::inputs::require_head(view, &inspection.state.head, grant)?;
    inspection.state.inputs.require_current(
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
