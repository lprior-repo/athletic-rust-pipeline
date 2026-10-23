use super::{codec, refused, PublicResourceDeclarationV1, PublicResourceFrontierEntryV1};
use super::{DECLARATION, ENTRY_PREFIX, FRONTIER_OPERATION_PREFIX};
use crate::acquisition::{AcquisitionReadBudget, CaptureError};
use crate::{Receipt, Store, StoreError, StoreFormat, StoreResult};
use census_domain::model::*;
use fjall::{Keyspace, Readable, Snapshot};
use serde::{de::DeserializeOwned, Serialize};

pub(crate) fn record<T: DeserializeOwned + Serialize>(
    snapshot: &Snapshot,
    table: &Keyspace,
    key: &[u8],
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<T>> {
    record_using(snapshot, table, key, budget, codec::decode)
}

fn record_using<T>(
    snapshot: &Snapshot,
    table: &Keyspace,
    key: &[u8],
    budget: &mut AcquisitionReadBudget,
    decode: impl FnOnce(&[u8]) -> StoreResult<T>,
) -> StoreResult<Option<T>> {
    budget.admission_probe()?;
    let Some(length) = snapshot
        .size_of(table, key)
        .map_err(|source| StoreError::Read { source })?
    else {
        return Ok(None);
    };
    let length = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
    if length > codec::MAX_BYTES || key.len() > crate::MAX_JOURNAL_KEY_BYTES {
        return Err(CaptureError::RecordTooLarge.into());
    }
    budget.public_owner_record(key.len(), length).inspect_err(|error| {
        tracing::warn!(physical_key = %String::from_utf8_lossy(key), physical_key_bytes = key.len(), encoded_bytes = length, %error, "public resource record admission refused");
    })?;
    let raw = snapshot
        .get(table, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| refused("public resource record disappeared"))?;
    crate::acquisition::codec::reserve_decode(&raw, budget)?;
    decode(&raw).map(Some)
}

pub(crate) fn declaration(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    format: &StoreFormat,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<PublicResourceDeclarationV1>> {
    let Some(declaration) = held_declaration(snapshot, journal, budget)? else {
        return Ok(None);
    };
    let capture = crate::acquisition::qualification::declaration_admitted(
        snapshot,
        meta,
        journal,
        format,
        &mut crate::acquisition::admission::ReadMeter::Recovery(budget),
    )?
    .ok_or(CaptureError::CaptureCapabilityMissing)?;
    finish_declaration(snapshot, meta, receipts, declaration, &capture, budget).map(Some)
}

pub(in crate::acquisition) fn declaration_after_capture(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    capture: &CaptureQualification,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<PublicResourceDeclarationV1>> {
    let Some(declaration) = held_declaration(snapshot, journal, budget)? else {
        return Ok(None);
    };
    finish_declaration(snapshot, meta, receipts, declaration, capture, budget).map(Some)
}

fn held_declaration(
    snapshot: &Snapshot,
    journal: &Keyspace,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<PublicResourceDeclarationV1>> {
    let declaration: Option<PublicResourceDeclarationV1> =
        record(snapshot, journal, DECLARATION.as_bytes(), budget)?;
    if let Some(declaration) = &declaration {
        declaration.validate_shape()?;
    }
    Ok(declaration)
}

fn finish_declaration(
    snapshot: &Snapshot,
    meta: &Keyspace,
    receipts: &Keyspace,
    declaration: PublicResourceDeclarationV1,
    capture: &CaptureQualification,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<PublicResourceDeclarationV1> {
    let manifest: RunManifest = record_using(
        snapshot,
        meta,
        crate::run::RUN_MANIFEST.as_bytes(),
        budget,
        |raw| {
            serde_json::from_slice(raw)
                .map_err(|_| refused("public resource run manifest decoding"))
        },
    )?
    .ok_or(CaptureError::CaptureRunManifestMissing)?;
    let profile_id = declaration.binding.selection.profile_id();
    let profile_key = crate::profiles::key(profile_id);
    let profile = record_using(snapshot, meta, profile_key.as_bytes(), budget, |raw| {
        crate::profiles::read::decode(profile_id, raw)
    })?
    .ok_or_else(|| refused("public resource selected profile absent"))?;
    let operation = declaration.receipt()?.operation;
    let receipt = record(snapshot, receipts, operation.as_bytes(), budget)?
        .ok_or_else(|| refused("public resource declaration receipt absent"))?;
    super::format::validate_declaration(&declaration, capture, &manifest, &profile, &receipt)?;
    Ok(declaration)
}

pub(crate) fn entry(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    key: &[u8],
    declaration: &PublicResourceDeclarationV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<PublicResourceFrontierEntryV1> {
    let entry = checked_entry(snapshot, journal, receipts, key, declaration, budget)?;
    links(snapshot, journal, &entry.work, budget)?;
    Ok(entry)
}

fn checked_entry(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    key: &[u8],
    declaration: &PublicResourceDeclarationV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<PublicResourceFrontierEntryV1> {
    let entry: PublicResourceFrontierEntryV1 = record(snapshot, journal, key, budget)?
        .ok_or_else(|| refused("public resource frontier entry absent"))?;
    super::format::validate_entry_key(key, &entry)?;
    let operation = format!("{FRONTIER_OPERATION_PREFIX}{}", entry.id()?);
    let receipt: Receipt = record(snapshot, receipts, operation.as_bytes(), budget)?
        .ok_or_else(|| refused("public resource frontier receipt absent"))?;
    super::format::validate_entry_receipt(declaration, &entry, &receipt)?;
    Ok(entry)
}

pub(crate) fn links(
    snapshot: &Snapshot,
    journal: &Keyspace,
    work: &AcquisitionWorkV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let protocol = CaptureProtocolVersion::V2;
    let id = work.id().map_err(CaptureError::Domain)?;
    let slot = work.slot().id().map_err(CaptureError::Domain)?;
    let binding: crate::acquisition::facts::SlotBinding = crate::acquisition::codec::get_fact(
        snapshot,
        journal,
        &format!("slot/{slot}/binding"),
        budget,
        protocol,
    )?
    .ok_or(CaptureError::SlotWorkMismatch)?;
    let held: AcquisitionWorkV1 = crate::acquisition::codec::get(
        snapshot,
        journal,
        &format!("work/{id}/identity"),
        budget,
        protocol,
    )?
    .ok_or(CaptureError::SlotWorkMismatch)?;
    if binding.version != protocol
        || binding.work_id != id
        || &binding.slot != work.slot()
        || &held != work
    {
        return Err(CaptureError::SlotWorkMismatch.into());
    }
    Ok(())
}

pub(crate) fn for_work(
    store: &Store,
    snapshot: &Snapshot,
    work: &AcquisitionWorkV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    validate_work(
        snapshot,
        &store.meta,
        &store.journal,
        &store.receipts,
        work,
        budget,
    )
}

pub(crate) fn validate_work(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    work: &AcquisitionWorkV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    bound_work(snapshot, meta, journal, receipts, work, budget)?;
    links(snapshot, journal, work, budget)
}

pub(in crate::acquisition) fn for_read_work(
    store: &Store,
    snapshot: &Snapshot,
    facts: &crate::acquisition::facts::WorkFacts,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    if facts.protocol != CaptureProtocolVersion::V2 {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
    bound_work(
        snapshot,
        &store.meta,
        &store.journal,
        &store.receipts,
        &facts.work,
        budget,
    )
}

fn bound_work(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    work: &AcquisitionWorkV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let format = crate::format::read_capture_format(snapshot, meta, budget)?;
    let declaration = declaration(snapshot, meta, journal, receipts, &format, budget)?
        .ok_or(CaptureError::CaptureCapabilityNamespaceUnsupported)?;
    if work.slot().public_resource().is_some_and(|resource| {
        resource.kind() == PublicSourceResourceKindV1::MileSplitAthleteProfile
    }) {
        return crate::acquisition::owner::profile_child::for_work(
            snapshot,
            meta,
            journal,
            receipts,
            &declaration,
            work,
            budget,
        );
    }
    if work
        .slot()
        .public_resource()
        .is_some_and(|resource| resource.kind() == PublicSourceResourceKindV1::MileSplitTeamRoster)
    {
        return super::child::for_work(snapshot, journal, receipts, &declaration, work, budget);
    }
    root_binding(snapshot, journal, receipts, &declaration, work, budget)
}

pub(in crate::acquisition) fn root_binding(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &PublicResourceDeclarationV1,
    work: &AcquisitionWorkV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let id = super::frontier::entry_id(declaration, work)?;
    let entry = checked_entry(
        snapshot,
        journal,
        receipts,
        format!("{ENTRY_PREFIX}{id}").as_bytes(),
        declaration,
        budget,
    )?;
    if &entry.work != work {
        return Err(CaptureError::ObligationConflict.into());
    }
    Ok(())
}
