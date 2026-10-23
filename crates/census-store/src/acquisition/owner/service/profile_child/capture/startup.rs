use super::{operation, require_receipt, PublicProfileCapturePacketV1, MAX_BYTES, PHASE};
use crate::acquisition::public_resources::{read as public, refused, PublicResourceDeclarationV1};
use crate::read::{
    for_each_retained_journal_root_with_admission, read_retained_journal_entry_raw_bounded,
    JournalNamespace, PhysicalRetainedJournalSource,
};
use crate::{AcquisitionReadBudget, JournalReadBudget, Receipt, StoreError, StoreResult};
use census_domain::model::AcquisitionDigest;
use fjall::{Keyspace, Readable, Snapshot};

pub(crate) fn read_packet(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    key: &str,
) -> StoreResult<Option<PublicProfileCapturePacketV1>> {
    AcquisitionDigest::parse(key).map_err(crate::CaptureError::Domain)?;
    let source = PhysicalRetainedJournalSource::new(snapshot, journal, receipts);
    let Some(envelope) = read_retained_journal_entry_raw_bounded(
        &source,
        PHASE,
        key,
        JournalReadBudget::new(MAX_BYTES)?,
    )?
    else {
        return Ok(None);
    };
    let packet = PublicProfileCapturePacketV1::decode(envelope.payload().get().as_bytes())?;
    if packet.reference()?.as_str() != key {
        return Err(refused("public profile capture root identity differs"));
    }
    let receipt: Receipt = public::record(
        snapshot,
        receipts,
        operation(key).as_bytes(),
        &mut AcquisitionReadBudget::for_work(),
    )?
    .ok_or_else(|| refused("public profile capture root receipt absent"))?;
    require_receipt(&packet, &receipt)?;
    if envelope.at() != receipt.at {
        return Err(refused("public profile capture root receipt time differs"));
    }
    Ok(Some(packet))
}

pub(crate) fn validate_all(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &PublicResourceDeclarationV1,
    maximum: usize,
) -> StoreResult<()> {
    let source = PhysicalRetainedJournalSource::new(snapshot, journal, receipts);
    let mut count = 0_usize;
    for_each_retained_journal_root_with_admission(
        &source,
        JournalNamespace::whole_phase(PHASE)?,
        super::scan_budget_for(maximum)?,
        &mut count,
        |_| Ok(()),
        |count, key| {
            if *count >= maximum {
                return Err(refused("public profile capture root inventory limit"));
            }
            let packet = read_packet(snapshot, journal, receipts, key)?
                .ok_or_else(|| refused("public profile capture root disappeared"))?;
            if packet.declaration != *declaration {
                return Err(refused("public profile capture declaration differs"));
            }
            let entry = super::super::read::entry(
                snapshot,
                meta,
                journal,
                receipts,
                declaration,
                super::super::read::key(packet.capture().work())?.as_bytes(),
                &mut AcquisitionReadBudget::for_work(),
            )?;
            if entry != packet.child {
                return Err(refused("public profile capture first child differs"));
            }
            *count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            Ok(())
        },
    )?;
    receipt_inventory(snapshot, journal, receipts, maximum, count)
}

fn receipt_inventory(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    maximum: usize,
    expected: usize,
) -> StoreResult<()> {
    let prefix = format!("{PHASE}:");
    let mut count = 0_usize;
    for (position, guard) in snapshot
        .prefix(receipts, prefix.as_bytes())
        .take(maximum.checked_add(1).ok_or(StoreError::CounterOverflow)?)
        .enumerate()
    {
        if position >= maximum {
            return Err(refused("public profile capture receipt inventory limit"));
        }
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let id = key
            .strip_prefix(prefix.as_bytes())
            .and_then(|id| std::str::from_utf8(id).ok())
            .ok_or_else(|| refused("public profile capture receipt namespace"))?;
        read_packet(snapshot, journal, receipts, id)?
            .ok_or_else(|| refused("public profile capture orphan receipt"))?;
        count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    }
    if count != expected {
        return Err(refused("public profile capture receipt count differs"));
    }
    Ok(())
}
