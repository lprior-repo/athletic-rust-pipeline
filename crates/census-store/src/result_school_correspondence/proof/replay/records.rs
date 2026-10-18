use super::super::super::issuance::ledger::Ledger;
use super::*;
pub(super) fn load(
    archive: &archive::Archive<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Ledger> {
    let count = archive.records.len();
    if count > MAX_RESULT_SCHOOL_RECORDS
        || count != archive.receipts.len()
        || u64::try_from(count).map_err(|_| Error::BudgetExceeded)? != archive.inventory.records
    {
        return Err(Error::Corrupt("source closure record/receipt inventory").into());
    }
    let proof: wire::Proof = codec::decode(archive.enrollment.get().as_bytes(), grant)?;
    super::super::enrollment::verify(&proof, &archive.store, grant)?;
    let enrollment: wire::Enrollment = codec::decode(&proof.enrollment, grant)?;
    let mut ledger = Ledger {
        proof,
        store: enrollment.command.store,
        initial: enrollment.initial_inventory,
        inventory: archive.inventory.clone(),
        records: Vec::new(),
        receipts: Vec::new(),
        contexts: Vec::new(),
        active_indices: Vec::new(),
        checked_roots: Vec::new(),
    };
    grant
        .charge_bytes(
            count
                .checked_mul(std::mem::size_of::<ResultSchoolReviewedRecordV1>())
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
    archive
        .records
        .iter()
        .zip(&archive.receipts)
        .try_for_each(|(record, receipt)| {
            let envelope = codec::decode_entry(record.get().as_bytes(), grant)?;
            if envelope.schema != "result-school-reviewed-entry-v1" {
                return Err(Error::Corrupt("reviewed entry schema").into());
            }
            let bytes = codec::encode(&envelope.record, grant)?;
            let record = decode_result_school_record_v1(&bytes, grant).map_err(Error::Domain)?;
            ledger.records.push(record);
            ledger
                .receipts
                .push(codec::decode(receipt.get().as_bytes(), grant)?);
            ledger.contexts.push(envelope.context);
            Ok::<_, StoreError>(())
        })?;
    ledger.validate(grant)?;
    Ok(ledger)
}
