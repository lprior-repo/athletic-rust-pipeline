use super::*;
use crate::read::JournalNamespace;
use crate::{JournalReadBudget, ReadBudget, MAX_JOURNAL_VALUE_BYTES};

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
    let mut records = Vec::new();
    let mut input_refs = Vec::new();
    let mut introductions = Vec::new();
    for key in keys {
        let passes = rows
            .checked_add(1)
            .and_then(|n| n.checked_mul(17))
            .ok_or(StoreError::CounterOverflow)?;
        grant.charge_steps(passes).map_err(Error::Domain)?;
        let envelope = view
            .journal_entry_raw_bounded(PHASE, &key, JournalReadBudget::new(measured)?)?
            .ok_or(Error::Corrupt(
                "attachment root missing from held inventory",
            ))?;
        let (record, introduced, inputs) = wire::decode(&envelope, grant)?;
        material::allocate(&mut records)?;
        material::allocate(&mut introductions)?;
        context::accept(view, &introductions, &introduced, &record, grant)?;
        material::allocate(&mut input_refs)?;
        input_refs.push(inputs);
        records.push(record);
        introductions.push(introduced);
    }
    let receipts = receipts::load(view, &records, grant)?;
    let inventory = domain::calculate_reviewed_reference_inventory_with_grant(&records, grant)
        .map_err(Error::Domain)?;
    let ledger = Ledger {
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

fn inventory<'s>(
    view: &StoreSnapshot<'s>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(JournalNamespaceInventory<'s>, Vec<String>)> {
    grant.charge_steps(2).map_err(Error::Domain)?;
    let mut keys = Vec::new();
    let inventory = view.for_each_journal_root_with_admission(
        JournalNamespace::whole_phase(PHASE)?,
        ReadBudget {
            rows: 8192,
            bytes: 4 * 1024 * 1024,
            record_bytes: MAX_JOURNAL_VALUE_BYTES,
        },
        grant,
        |grant| {
            grant
                .charge_steps(4)
                .map_err(|source| Error::Domain(source).into())
        },
        |grant, key| {
            grant.charge_steps(1).map_err(Error::Domain)?;
            let ordinal = u64::try_from(keys.len())
                .map_err(|_| StoreError::CounterOverflow)?
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?;
            if keys::ordinal(key)? != ordinal {
                return Err(Error::Corrupt("noncontiguous attachment records").into());
            }
            grant.charge_bytes(key.len()).map_err(Error::Domain)?;
            material::allocate(&mut keys)?;
            let mut owned = String::new();
            owned
                .try_reserve_exact(key.len())
                .map_err(|_| Error::Domain(domain::EvidenceSubjectAttachmentError::Allocation))?;
            owned.push_str(key);
            keys.push(owned);
            Ok(())
        },
    )?;
    Ok((inventory, keys))
}
