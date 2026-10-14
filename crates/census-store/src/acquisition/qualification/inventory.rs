use crate::acquisition::{codec, facts::*, read, AcquisitionReadBudget, CaptureError};
use crate::{Store, StoreError, StoreResult};
use census_domain::model::*;
use fjall::{Keyspace, Readable, Snapshot};
use std::path::Path;

pub(super) fn inventory(
    root: &Path,
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &CaptureQualificationV1,
) -> StoreResult<()> {
    let mut budget = AcquisitionReadBudget::inventory();
    let manifest = inventory_manifest(snapshot, meta, &mut budget)?;
    let prefix = Store::journal_key(SOURCE_ACQUISITION_PHASE, "");
    for (position, guard) in snapshot
        .prefix(journal, b"source_acquisition_attempt_")
        .take(1_048_577)
        .enumerate()
    {
        if position == 1_048_576 {
            return Err(CaptureError::ReadBudgetExceeded {
                resource: "inventory roots",
            }
            .into());
        }
        let physical = guard.key().map_err(|source| StoreError::Read { source })?;
        if physical.len() > crate::MAX_JOURNAL_KEY_BYTES {
            return Err(CaptureError::UnsupportedRoot.into());
        }
        budget.encoded(physical.len())?;
        let logical = physical
            .strip_prefix(prefix.as_slice())
            .ok_or(CaptureError::CaptureCapabilityNamespaceUnsupported)?;
        let key = std::str::from_utf8(logical).map_err(|_| CaptureError::UnsupportedRoot)?;
        let parts: Vec<&str> = key.split('/').take(8).collect();
        match parts.as_slice() {
            ["slot", slot, "binding"] => {
                let binding: SlotBinding = codec::get(snapshot, journal, key, &mut budget)?
                    .ok_or(CaptureError::SlotWorkMismatch)?;
                if binding.slot.id().map_err(CaptureError::Domain)?.as_str() != *slot
                    || binding.slot.store_identity() != declaration.store_identity()
                {
                    return Err(CaptureError::SlotWorkMismatch.into());
                }
                let identity = format!("work/{}/identity", binding.work_id);
                let work: AcquisitionWorkV1 =
                    codec::get(snapshot, journal, &identity, &mut budget)?
                        .ok_or(CaptureError::SlotWorkMismatch)?;
                if work.slot() != &binding.slot
                    || work.id().map_err(CaptureError::Domain)? != binding.work_id
                {
                    return Err(CaptureError::SlotWorkMismatch.into());
                }
            }
            ["work", id, "identity"] => {
                let id = AcquisitionWorkId::parse(id).map_err(CaptureError::Domain)?;
                let facts = read::read_work(snapshot, journal, receipts, &id, &mut budget)?;
                let (manifest, manifest_digest) = manifest
                    .as_ref()
                    .ok_or(CaptureError::CaptureRunManifestMissing)?;
                if manifest.store_identity != declaration.store_identity().as_str()
                    || facts.work.run() != manifest.run
                    || facts.work.run_manifest_digest() != manifest_digest
                    || manifest.cohort.get() != 2027
                {
                    return Err(CaptureError::CaptureRunMismatch.into());
                }
                if facts.work.slot().store_identity() != declaration.store_identity() {
                    return Err(CaptureError::CaptureCapabilityForeignStore.into());
                }
                validate_backing(root, snapshot, journal, receipts, &facts, &mut budget)?;
            }
            ["work", id, ..] => {
                let id = AcquisitionWorkId::parse(id).map_err(CaptureError::Domain)?;
                let identity =
                    Store::journal_key(SOURCE_ACQUISITION_PHASE, &format!("work/{id}/identity"));
                if !snapshot
                    .contains_key(journal, &identity)
                    .map_err(|source| StoreError::Read { source })?
                {
                    return Err(CaptureError::SlotWorkMismatch.into());
                }
            }
            _ => return Err(CaptureError::UnsupportedRoot.into()),
        }
    }
    inventory_receipts(snapshot, journal, receipts, &mut budget)?;
    crate::acquisition::cache::inventory(
        root,
        snapshot,
        journal,
        receipts,
        declaration,
        &mut budget,
    )?;
    Ok(())
}
fn inventory_receipts(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    for (position, guard) in snapshot
        .prefix(receipts, b"source_acquisition_attempt_")
        .take(1_048_577)
        .enumerate()
    {
        if position == 1_048_576 {
            return Err(CaptureError::ReadBudgetExceeded {
                resource: "receipt inventory",
            }
            .into());
        }
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        if key.len() > crate::MAX_OPERATION_BYTES {
            return Err(CaptureError::UnsupportedRoot.into());
        }
        budget.encoded(key.len())?;
        let key = std::str::from_utf8(&key).map_err(|_| CaptureError::UnsupportedRoot)?;
        let Some(logical) = key.strip_prefix("source_acquisition_attempt_v1/") else {
            return Err(CaptureError::CaptureCapabilityNamespaceUnsupported.into());
        };
        let parts: Vec<&str> = logical.split('/').take(6).collect();
        let ["work", work, "attempt", ordinal, "terminal"] = parts.as_slice() else {
            return Err(CaptureError::UnsupportedRoot.into());
        };
        AcquisitionWorkId::parse(work).map_err(CaptureError::Domain)?;
        if !matches!(*ordinal, "1" | "2" | "3") {
            return Err(CaptureError::UnsupportedRoot.into());
        }
        if !snapshot
            .contains_key(
                journal,
                Store::journal_key(SOURCE_ACQUISITION_PHASE, logical),
            )
            .map_err(|source| StoreError::Read { source })?
        {
            return Err(CaptureError::CorruptClosure.into());
        }
    }
    Ok(())
}
fn validate_backing(
    root: &Path,
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    facts: &WorkFacts,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    for attempt in &facts.attempts {
        if let Some(terminal) = &attempt.terminal {
            match terminal.terminal() {
                AcquisitionTerminalV1::ResponseRetained { .. } => {
                    let prepared = attempt
                        .prepared
                        .as_ref()
                        .ok_or(CaptureError::CorruptClosure)?;
                    crate::acquisition::backing::verify(
                        root,
                        &facts.work,
                        attempt,
                        &prepared.capture,
                        budget,
                    )?;
                }
                AcquisitionTerminalV1::ValidatedNotModified { original, .. } => {
                    let original_facts = read::read_work(
                        snapshot,
                        journal,
                        receipts,
                        original.attempt().work_id(),
                        budget,
                    )?;
                    if facts.work.request() != original_facts.work.request()
                        || original.attempt().store_identity() != facts.work.slot().store_identity()
                    {
                        return Err(CaptureError::CacheOccurrenceMismatch.into());
                    }
                    read::reusable_body_reference(root, &original_facts, original, budget)?;
                }
                _ => {}
            }
        }
    }
    if let Some(result) = &facts.result {
        if result.occurrence.attempt().work_id()
            != &facts.work.id().map_err(CaptureError::Domain)?
        {
            read::verify_result(
                root,
                snapshot,
                journal,
                receipts,
                facts,
                &result.occurrence,
                budget,
            )?;
        }
    }
    Ok(())
}

fn inventory_manifest(
    snapshot: &Snapshot,
    meta: &Keyspace,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<(RunManifest, AcquisitionDigest)>> {
    if let Some(length) = snapshot
        .size_of(meta, crate::run::RUN_MANIFEST)
        .map_err(|source| StoreError::Read { source })?
    {
        let length = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
        if length > MAX_CAPTURE_RECORD_BYTES {
            return Err(CaptureError::RecordTooLarge.into());
        }
        budget.encoded(
            length
                .checked_add(crate::run::RUN_MANIFEST.len())
                .ok_or(StoreError::CounterOverflow)?,
        )?;
    }
    crate::run::read(snapshot, meta)?
        .map(|manifest| {
            let digest = digest(&manifest)?;
            Ok((manifest, digest))
        })
        .transpose()
}
