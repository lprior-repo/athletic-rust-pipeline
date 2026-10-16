use super::*;
use crate::read::{read_retained_journal_entry_raw_bounded, JournalNamespaceContentsInventory};
use crate::JournalReadBudget;

pub(super) use super::inventory::live as inventory;

pub(super) fn load<'s>(
    view: &StoreSnapshot<'s>,
    store: &StoreIdentity,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Ledger<'s>> {
    let (physical, keys) = inventory(view, grant)?;
    let measured = usize::try_from(physical.fingerprint().encoded_bytes())
        .map_err(|_| StoreError::CounterOverflow)?;
    let rows = usize::try_from(physical.fingerprint().physical_rows())
        .map_err(|_| StoreError::CounterOverflow)?;
    load_acquired(view, store, physical, keys, (measured, rows), grant)
}

pub(in super::super) fn retained(
    source: &impl RetainedJournalReadSource,
    store: &StoreIdentity,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<VerifiedLedger<JournalNamespaceContentsInventory>> {
    let (physical, keys) = super::inventory::retained(source, grant)?;
    let measured = usize::try_from(physical.contents().encoded_bytes())
        .map_err(|_| StoreError::CounterOverflow)?;
    let rows = usize::try_from(physical.contents().physical_rows())
        .map_err(|_| StoreError::CounterOverflow)?;
    load_acquired(source, store, physical, keys, (measured, rows), grant)
}

fn load_acquired<P>(
    source: &impl RetainedJournalReadSource,
    store: &StoreIdentity,
    physical: P,
    keys: Vec<String>,
    bounds: (usize, usize),
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<VerifiedLedger<P>> {
    let (measured, rows) = bounds;
    let mut records = CheckedReviewedReferenceRecordsV1::new();
    let mut input_refs = Vec::new();
    let mut introductions = Vec::new();
    for key in keys {
        let passes = rows
            .checked_add(1)
            .and_then(|n| n.checked_mul(17))
            .ok_or(StoreError::CounterOverflow)?;
        grant.charge_steps(passes).map_err(Error::Domain)?;
        let envelope = read_retained_journal_entry_raw_bounded(
            source,
            PHASE,
            &key,
            JournalReadBudget::new(measured)?,
        )?
        .ok_or(Error::Corrupt(
            "attachment root missing from held inventory",
        ))?;
        let (record, introduced, inputs) = wire::decode(&envelope, grant)?;
        material::allocate(&mut introductions)?;
        context::accept(source, &introductions, &introduced, &record, grant)?;
        material::allocate(&mut input_refs)?;
        input_refs.push(inputs);
        records
            .push_with_grant(record, grant)
            .map_err(Error::Domain)?;
        introductions.push(introduced);
    }
    let receipts = receipts::load(source, &records, grant)?;
    let inventory = domain::calculate_reviewed_reference_inventory_with_grant(&records, grant)
        .map_err(Error::Domain)?;
    let ledger = VerifiedLedger {
        records,
        receipts,
        introductions,
        input_refs,
        inventory,
        physical,
    };
    ledger.validate_projection(store, grant)?;
    for (record, inputs) in ledger.records.iter().zip(&ledger.input_refs) {
        fences::verify(&ledger, record, inputs, grant)?;
        super::super::target::verify_historical(&ledger, record, grant)?;
    }
    Ok(ledger)
}
