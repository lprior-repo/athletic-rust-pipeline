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
    declaration: &CaptureQualification,
) -> StoreResult<()> {
    let protocol = declaration.protocol();
    let mut budget = AcquisitionReadBudget::inventory();
    let manifest = inventory_manifest(snapshot, meta, &mut budget)?;
    let prefix = Store::journal_key(protocol.phase(), "");
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
        let key = inventory_key(&physical, &prefix, &mut budget)?;
        let parts: Vec<&str> = key.split('/').take(8).collect();
        match parts.as_slice() {
            ["slot", slot, "binding"] => {
                inventory_slot(snapshot, journal, key, slot, declaration, &mut budget)?;
            }
            ["work", id, "identity"] => {
                let id = AcquisitionWorkId::parse(id).map_err(CaptureError::Domain)?;
                let facts =
                    read::read_work(snapshot, journal, receipts, &id, &mut budget, protocol)?;
                validate_manifest(declaration, &facts, manifest.as_ref())?;
                validate_backing(root, snapshot, journal, receipts, &facts, &mut budget)?;
            }
            ["work", id, ..] => {
                let id = AcquisitionWorkId::parse(id).map_err(CaptureError::Domain)?;
                require_work_identity(snapshot, journal, &id, protocol)?;
            }
            _ => return Err(CaptureError::UnsupportedRoot.into()),
        }
    }
    inventory_receipts(snapshot, journal, receipts, &mut budget, protocol)?;
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
fn inventory_key<'a>(
    physical: &'a [u8],
    prefix: &[u8],
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<&'a str> {
    if physical.len() > crate::MAX_JOURNAL_KEY_BYTES {
        return Err(CaptureError::UnsupportedRoot.into());
    }
    budget.encoded(physical.len())?;
    let logical = physical
        .strip_prefix(prefix)
        .ok_or(CaptureError::CaptureCapabilityNamespaceUnsupported)?;
    std::str::from_utf8(logical).map_err(|_| CaptureError::UnsupportedRoot.into())
}

fn inventory_slot(
    snapshot: &Snapshot,
    journal: &Keyspace,
    key: &str,
    slot: &str,
    declaration: &CaptureQualification,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let protocol = declaration.protocol();
    let binding: SlotBinding = codec::get_fact(snapshot, journal, key, budget, protocol)?
        .ok_or(CaptureError::SlotWorkMismatch)?;
    if binding.version != protocol {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
    if binding.slot.id().map_err(CaptureError::Domain)?.as_str() != slot
        || binding.slot.store_identity() != declaration.store_identity()
    {
        return Err(CaptureError::SlotWorkMismatch.into());
    }
    let identity = format!("work/{}/identity", binding.work_id);
    let work: AcquisitionWorkV1 = codec::get(snapshot, journal, &identity, budget, protocol)?
        .ok_or(CaptureError::SlotWorkMismatch)?;
    if work.slot() != &binding.slot || work.id().map_err(CaptureError::Domain)? != binding.work_id {
        return Err(CaptureError::SlotWorkMismatch.into());
    }
    Ok(())
}
fn validate_manifest(
    declaration: &CaptureQualification,
    facts: &WorkFacts,
    manifest: Option<&(RunManifest, AcquisitionDigest)>,
) -> StoreResult<()> {
    let (manifest, manifest_digest) = manifest.ok_or(CaptureError::CaptureRunManifestMissing)?;
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
    Ok(())
}
fn require_work_identity(
    snapshot: &Snapshot,
    journal: &Keyspace,
    id: &AcquisitionWorkId,
    protocol: CaptureProtocolVersion,
) -> StoreResult<()> {
    let identity = Store::journal_key(protocol.phase(), &format!("work/{id}/identity"));
    if !snapshot
        .contains_key(journal, &identity)
        .map_err(|source| StoreError::Read { source })?
    {
        return Err(CaptureError::SlotWorkMismatch.into());
    }
    Ok(())
}
fn inventory_receipts(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    budget: &mut AcquisitionReadBudget,
    protocol: CaptureProtocolVersion,
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
        let Some(logical) = key.strip_prefix(&format!("{}/", protocol.phase())) else {
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
            .contains_key(journal, Store::journal_key(protocol.phase(), logical))
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
                AcquisitionTerminal::ResponseRetained { .. } => {
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
                AcquisitionTerminal::ValidatedNotModified { original, .. } => {
                    let original_facts = read::read_work(
                        snapshot,
                        journal,
                        receipts,
                        original.attempt().work_id(),
                        budget,
                        facts.protocol,
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
    verify_reused_result(root, snapshot, journal, receipts, facts, budget)
}

fn verify_reused_result(
    root: &Path,
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    facts: &WorkFacts,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
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
