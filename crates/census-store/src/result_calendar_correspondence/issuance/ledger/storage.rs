use super::*;
use fjall::{Keyspace, Readable, Snapshot};
pub(in super::super::super) fn load(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    proof: wire::Proof,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Ledger> {
    let enrollment: wire::Enrollment = codec::decode(proof.enrollment.get().as_bytes(), grant)?;
    let (count, head) = validate_head(snapshot, meta, grant)?;
    let mut ledger = build_ledger_skeleton(proof, enrollment, head)?;
    charge_and_reserve(&mut ledger, count, grant)?;
    load_all_records(snapshot, journal, receipts, &mut ledger, count, grant)?;
    receipt_namespace(snapshot, receipts, &ledger, grant)?;
    ledger.validate(grant)?;
    Ok(ledger)
}

fn validate_head(
    snapshot: &Snapshot,
    meta: &Keyspace,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(usize, wire::AuthorityHead)> {
    let raw = read::value(snapshot, meta, wire::HEAD.as_bytes(), grant)?;
    let head: wire::AuthorityHead = codec::decode(&raw, grant)?;
    if head.schema != "result-calendar-authority-head-v1"
        || head.inventory.version != 1
        || head.inventory.records
            > u64::try_from(MAX_RESULT_CALENDAR_RECORDS).map_err(|_| Error::BudgetExceeded)?
    {
        return Err(Error::Corrupt("authority head").into());
    }
    let count = usize::try_from(head.inventory.records).map_err(|_| Error::BudgetExceeded)?;
    Ok((count, head))
}

fn build_ledger_skeleton(
    proof: wire::Proof,
    enrollment: wire::Enrollment,
    head: wire::AuthorityHead,
) -> StoreResult<Ledger> {
    Ok(Ledger {
        store: enrollment.command.store,
        initial: enrollment.initial_inventory,
        inventory: head.inventory,
        records: Vec::new(),
        receipts: Vec::new(),
        contexts: Vec::new(),
        active_indices: Vec::new(),
        checked_roots: Vec::new(),
        proof,
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
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    ledger: &mut Ledger,
    count: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let mut keys = snapshot.prefix(journal, NAME);
    (0..count).try_for_each(|ordinal| -> StoreResult<()> {
        grant.charge_steps(1).map_err(grant_error)?;
        let key = keys
            .next()
            .ok_or(Error::Corrupt("missing issuance record"))?
            .key()
            .map_err(|source| StoreError::Read { source })?;
        if key.as_ref() != wire::record_key(ordinal)?.as_bytes() {
            return Err(Error::Corrupt("record ordinal or namespace").into());
        }
        load_single_record(snapshot, journal, receipts, &key, ledger, grant)
    })?;
    if let Some(row) = keys.next() {
        row.key().map_err(|source| StoreError::Read { source })?;
        return Err(Error::Corrupt("extra issuance namespace row").into());
    }
    Ok(())
}

fn load_single_record(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    key: &[u8],
    ledger: &mut Ledger,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let raw = read::entry_value(snapshot, journal, key, grant)?;
    let envelope: wire::RecordEnvelope = codec::decode_bundle(&raw, grant)?;
    if envelope.schema != "result-calendar-review-record-envelope-v1" {
        return Err(Error::Corrupt("reviewed entry schema").into());
    }
    let record_bytes = codec::encode(&envelope.record, grant)?;
    let record = decode_result_calendar_record_v1(&record_bytes, grant).map_err(Error::Domain)?;
    let receipt = read::value(
        snapshot,
        receipts,
        wire::issuance_operation(&record.command).as_bytes(),
        grant,
    )?;
    ledger.receipts.push(codec::decode(&receipt, grant)?);
    ledger.records.push(record);
    ledger.contexts.push(envelope.context);
    Ok(())
}

fn receipt_namespace(
    snapshot: &Snapshot,
    receipts: &Keyspace,
    ledger: &Ledger,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let enrollment: wire::Enrollment =
        codec::decode(ledger.proof.enrollment.get().as_bytes(), grant)?;
    let enrollment_operation = wire::enrollment_operation(&enrollment.command);
    let expected = ledger
        .records
        .len()
        .checked_add(1)
        .ok_or(Error::BudgetExceeded)?;
    let mut seen = 0usize;
    snapshot
        .prefix(receipts, NAME)
        .take(MAX_RESULT_CALENDAR_RECORDS + 2)
        .try_for_each(|row| -> StoreResult<()> {
            grant.charge_steps(1).map_err(grant_error)?;
            let key = row.key().map_err(|source| StoreError::Read { source })?;
            grant.charge_bytes(key.len()).map_err(grant_error)?;
            if key.as_ref() != enrollment_operation.as_bytes()
                && !ledger
                    .receipts
                    .iter()
                    .any(|receipt| receipt.operation.as_bytes() == key.as_ref())
            {
                return Err(Error::Corrupt("unexpected issuance receipt").into());
            }
            seen = seen.checked_add(1).ok_or(Error::BudgetExceeded)?;
            if seen > expected {
                return Err(Error::Corrupt("extra issuance receipt").into());
            }
            Ok(())
        })?;
    if seen != expected {
        return Err(Error::Corrupt("missing issuance receipt").into());
    }
    Ok(())
}
