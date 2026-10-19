use super::super::super::issuance::ledger::Ledger;
use super::*;
pub(super) fn load(
    archive: &archive::Archive<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Ledger> {
    let count = validate_inventory(archive)?;
    let proof = decode_enrollment(archive, grant)?;
    let mut ledger = build_ledger_skeleton(proof, archive, grant)?;
    charge_and_reserve(&mut ledger, count, grant)?;
    load_all_records(archive, &mut ledger, grant)?;
    ledger.validate(grant)?;
    Ok(ledger)
}
fn validate_inventory(archive: &archive::Archive<'_>) -> StoreResult<usize> {
    let count = archive.records.len();
    if count > MAX_RESULT_CALENDAR_RECORDS
        || count != archive.receipts.len()
        || u64::try_from(count).map_err(|_| Error::BudgetExceeded)? != archive.inventory.records
    {
        return Err(Error::Corrupt("source closure record/receipt inventory").into());
    }
    Ok(count)
}

fn decode_enrollment(
    archive: &archive::Archive<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<wire::Proof> {
    let proof: wire::Proof = codec::decode(archive.enrollment.get().as_bytes(), grant)?;
    super::super::enrollment::verify(&proof, &archive.store, grant)?;
    Ok(proof)
}

fn build_ledger_skeleton(
    proof: wire::Proof,
    archive: &archive::Archive<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Ledger> {
    let enrollment: wire::Enrollment = codec::decode(proof.enrollment.get().as_bytes(), grant)?;
    Ok(Ledger {
        proof,
        store: enrollment.command.store,
        initial: enrollment.initial_inventory,
        inventory: archive.inventory.clone(),
        records: Vec::new(),
        receipts: Vec::new(),
        contexts: Vec::new(),
        active_indices: Vec::new(),
        checked_roots: Vec::new(),
    })
}

fn charge_and_reserve(
    ledger: &mut Ledger,
    count: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant
        .charge_bytes(
            count
                .checked_mul(std::mem::size_of::<ResultCalendarReviewedRecordV1>())
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    ledger
        .records
        .try_reserve_exact(count)
        .map_err(|_| Error::BudgetExceeded)?;
    ledger
        .receipts
        .try_reserve_exact(count)
        .map_err(|_| Error::BudgetExceeded)?;
    ledger
        .contexts
        .try_reserve_exact(count)
        .map_err(|_| Error::BudgetExceeded)?;
    Ok(())
}

fn load_all_records(
    archive: &archive::Archive<'_>,
    ledger: &mut Ledger,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    archive
        .records
        .iter()
        .zip(&archive.receipts)
        .try_for_each(|(record, receipt)| load_single_record(record, receipt, ledger, grant))
}

fn load_single_record(
    record: &serde_json::value::RawValue,
    receipt: &serde_json::value::RawValue,
    ledger: &mut Ledger,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let envelope: wire::RecordEnvelope = codec::decode_bundle(record.get().as_bytes(), grant)?;
    if envelope.schema != "result-calendar-review-record-envelope-v1" {
        return Err(Error::Corrupt("reviewed entry schema").into());
    }
    let bytes = codec::encode(&envelope.record, grant)?;
    let record = decode_result_calendar_record_v1(&bytes, grant).map_err(Error::Domain)?;
    ledger.records.push(record);
    ledger
        .receipts
        .push(codec::decode(receipt.get().as_bytes(), grant)?);
    ledger.contexts.push(envelope.context);
    Ok(())
}
