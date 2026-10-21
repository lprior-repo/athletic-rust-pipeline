use super::{packet::Wire, refused, PublicSchoolIntakeCaptureV1, PublicSchoolIntakePacketV1};
use crate::acquisition::public_resources::{child, read as public, ENTRY_PREFIX};
use crate::acquisition::{facts::*, read as acquisition, AcquisitionReadBudget};
use crate::{PublicResourceCaptureOwnerV1, Receipt, Store, StoreResult};
use census_domain::model::*;

mod body;

pub(super) fn derive(
    store: &Store,
    owner: &PublicResourceCaptureOwnerV1,
    directory: &AcquisitionOccurrenceRefV1,
    roster: &AcquisitionOccurrenceRefV1,
) -> StoreResult<PublicSchoolIntakePacketV1> {
    let mut budget = AcquisitionReadBudget::for_work();
    let declaration = owner.declaration(store, &mut budget)?;
    let directory = capture(store, owner, directory)?;
    let roster = capture(store, owner, roster)?;
    let snapshot = store.db.snapshot();
    let frontier_id = super::super::frontier::entry_id(&declaration, directory.work())?;
    let directory_frontier = public::entry(
        &snapshot,
        &store.journal,
        &store.receipts,
        format!("{ENTRY_PREFIX}{frontier_id}").as_bytes(),
        &declaration,
        &mut budget,
    )?;
    let child = child::validate_entry(
        &snapshot,
        &store.journal,
        &store.receipts,
        child::key(roster.work())?.as_bytes(),
        &declaration,
        &mut budget,
    )?;
    let declaration_receipt = receipt(store, &declaration.receipt()?.operation, &mut budget)?;
    let directory_receipt = receipt(
        store,
        &format!("{}{frontier_id}", super::super::FRONTIER_OPERATION_PREFIX),
        &mut budget,
    )?;
    let child_receipt = receipt(store, &child.operation()?, &mut budget)?;
    PublicSchoolIntakePacketV1::from_wire(Wire {
        version: 1,
        interpretation: super::INTERPRETATION.into(),
        declaration,
        declaration_receipt,
        directory_frontier,
        directory_receipt,
        child,
        child_receipt,
        directory,
        roster,
    })
}

fn receipt(
    store: &Store,
    operation: &str,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Receipt> {
    public::record(
        &store.db.snapshot(),
        &store.receipts,
        operation.as_bytes(),
        budget,
    )?
    .ok_or_else(|| refused("public school intake source receipt absent"))
}

fn capture(
    store: &Store,
    owner: &PublicResourceCaptureOwnerV1,
    reference: &AcquisitionOccurrenceRefV1,
) -> StoreResult<PublicSchoolIntakeCaptureV1> {
    let snapshot = store.db.snapshot();
    let mut budget = AcquisitionReadBudget::for_work();
    let facts = acquisition::read_work(
        &snapshot,
        &store.journal,
        &store.receipts,
        reference.attempt().work_id(),
        &mut budget,
        CaptureProtocolVersion::V2,
    )?;
    owner.matches_work(&facts.work)?;
    if facts.result.as_ref().map(|result| &result.occurrence) != Some(reference) {
        return Err(refused("public school intake original is not work result"));
    }
    let verified =
        acquisition::reusable_body_reference(&store.root, &facts, reference, &mut budget)?;
    let attempt = facts
        .attempts
        .iter()
        .find(|attempt| &attempt.reservation.attempt == reference.attempt())
        .ok_or_else(|| refused("public school intake original attempt absent"))?;
    let prepared = attempt
        .prepared
        .clone()
        .ok_or_else(|| refused("public school intake original prepared absent"))?;
    let terminal = attempt
        .terminal
        .clone()
        .ok_or_else(|| refused("public school intake original terminal absent"))?;
    let ReceiptWitness::Present { receipt, .. } = &attempt.receipt else {
        return Err(refused(
            "public school intake original terminal receipt absent",
        ));
    };
    let result = facts
        .result
        .clone()
        .ok_or_else(|| refused("public school intake original result absent"))?;
    let value = PublicSchoolIntakeCaptureV1 {
        work: facts.work.clone(),
        prepared,
        terminal,
        terminal_receipt: receipt.clone(),
        result,
        response: verified.response().clone(),
        occurrence: reference.clone(),
        body: body::read(verified.body_path())?,
    };
    value.validate()?;
    Ok(value)
}
