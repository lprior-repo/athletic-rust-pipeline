use super::*;
use crate::read::{
    for_each_retained_journal_root_with_admission, JournalNamespace,
    JournalNamespaceContentsInventory,
};
use crate::{ReadBudget, MAX_JOURNAL_VALUE_BYTES};

fn budget() -> ReadBudget {
    ReadBudget {
        rows: 8192,
        bytes: 4 * 1024 * 1024,
        record_bytes: MAX_JOURNAL_VALUE_BYTES,
    }
}

pub(super) fn live<'s>(
    view: &StoreSnapshot<'s>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(JournalNamespaceInventory<'s>, Vec<String>)> {
    grant.charge_steps(2).map_err(Error::Domain)?;
    let mut keys = Vec::new();
    let inventory = view.for_each_journal_root_with_admission(
        JournalNamespace::whole_phase(PHASE)?,
        budget(),
        grant,
        admit_row,
        |grant, key| accept_key(&mut keys, key, grant),
    )?;
    Ok((inventory, keys))
}

pub(super) fn retained(
    source: &impl RetainedJournalReadSource,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(JournalNamespaceContentsInventory, Vec<String>)> {
    grant.charge_steps(2).map_err(Error::Domain)?;
    let mut keys = Vec::new();
    let inventory = for_each_retained_journal_root_with_admission(
        source,
        JournalNamespace::whole_phase(PHASE)?,
        budget(),
        grant,
        admit_row,
        |grant, key| accept_key(&mut keys, key, grant),
    )?;
    Ok((inventory, keys))
}

fn admit_row(grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
    grant
        .charge_steps(4)
        .map_err(|source| Error::Domain(source).into())
}

fn accept_key(
    keys: &mut Vec<String>,
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    let ordinal = u64::try_from(keys.len())
        .map_err(|_| StoreError::CounterOverflow)?
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    if keys::ordinal(key)? != ordinal {
        return Err(Error::Corrupt("noncontiguous attachment records").into());
    }
    grant.charge_bytes(key.len()).map_err(Error::Domain)?;
    material::allocate(keys)?;
    let mut owned = String::new();
    owned
        .try_reserve_exact(key.len())
        .map_err(|_| Error::Domain(domain::EvidenceSubjectAttachmentError::Allocation))?;
    owned.push_str(key);
    keys.push(owned);
    Ok(())
}
