use super::{CaptureError, CaptureWriterV1};
use crate::{Store, StoreError, StoreFormat, StoreResult};
use census_domain::model::*;
use fjall::{Database, Keyspace, Readable, Snapshot};
use std::path::Path;

mod inventory;
mod preservation;
use inventory::inventory;
pub(crate) use preservation::validate_preservation;

pub struct CaptureQualificationControl {
    store: Store,
}
pub struct CaptureQualificationStore {
    store: Store,
    writer: CaptureWriterV1,
}
impl CaptureQualificationControl {
    pub(crate) fn new(store: Store) -> Self {
        Self { store }
    }
    pub fn show(&self) -> StoreResult<CaptureQualification> {
        validate_format(
            self.store.root(),
            &self.store.db.snapshot(),
            &self.store.meta,
            &self.store.journal,
            &self.store.receipts,
            &self.store.store_format()?,
        )?
        .ok_or_else(|| CaptureError::CaptureCapabilityMissing.into())
    }
}
impl CaptureQualificationStore {
    pub(crate) fn new(store: Store, record: CaptureQualification) -> Self {
        let writer = CaptureWriterV1 {
            origin: store.appends.clone(),
            identity: record.store_identity().clone(),
            protocol: record.protocol(),
        };
        Self { store, writer }
    }
    pub fn into_parts(self) -> (Store, CaptureWriterV1) {
        (self.store, self.writer)
    }
}

pub(crate) fn require_legacy(record: &Option<CaptureQualification>) -> StoreResult<()> {
    if record.is_some() {
        return Err(CaptureError::CaptureQualificationOptInRequired.into());
    }
    Ok(())
}
pub(crate) fn validate_format(
    root: &Path,
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    format: &StoreFormat,
) -> StoreResult<Option<CaptureQualification>> {
    super::public_resources::format::validate_format(snapshot, meta, journal, receipts, format)?;
    let declaration = declaration(snapshot, meta, journal, format)?;
    if declaration.is_none()
        && snapshot
            .prefix(receipts, b"source_acquisition_attempt_")
            .next()
            .is_some()
    {
        return Err(CaptureError::CaptureMarkerMismatch.into());
    }
    if let Some(record) = &declaration {
        inventory(root, snapshot, meta, journal, receipts, record)?;
    }
    Ok(declaration)
}

pub(crate) fn declaration(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    format: &StoreFormat,
) -> StoreResult<Option<CaptureQualification>> {
    declaration_admitted(
        snapshot,
        meta,
        journal,
        format,
        &mut super::admission::ReadMeter::Legacy,
    )
}

pub(crate) fn declaration_admitted(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    format: &StoreFormat,
    meter: &mut super::admission::ReadMeter<'_>,
) -> StoreResult<Option<CaptureQualification>> {
    let mut rows = snapshot.prefix(meta, b"capture:");
    meter.probe()?;
    let row = rows
        .next()
        .map(|guard| guard.key().map_err(|source| StoreError::Read { source }))
        .transpose()?;
    meter.probe()?;
    if let Some(guard) = rows.next() {
        guard.key().map_err(|source| StoreError::Read { source })?;
        return Err(CaptureError::CaptureCapabilityNamespaceUnsupported.into());
    }
    meter.probe()?;
    let family = snapshot
        .prefix(journal, b"source_acquisition_attempt_")
        .next()
        .map(|guard| guard.key().map_err(|source| StoreError::Read { source }))
        .transpose()?;
    let graph = crate::public_identity::is_graph_qualification_lifecycle(format.lifecycle_revision);
    if format.lifecycle_revision != Some(CAPTURE_QUALIFICATION_LIFECYCLE) && !graph {
        if row.is_some() || family.is_some() {
            return Err(CaptureError::CaptureMarkerMismatch.into());
        }
        return Ok(None);
    }
    if graph && !graph_capture_present(row.as_deref(), family.is_some())? {
        return Ok(None);
    }
    if declaration_format_unsupported(format) {
        return Err(CaptureError::CaptureMarkerMismatch.into());
    }
    meter.probe()?;
    if snapshot.prefix(meta, b"installation:").next().is_some() {
        return Err(CaptureError::CaptureMarkerMismatch.into());
    }
    let row = row.ok_or(CaptureError::CaptureCapabilityMissing)?;
    let protocol = match row.as_ref() {
        b"capture:v1:qualification" => CaptureProtocolVersion::V1,
        b"capture:v2:qualification" => CaptureProtocolVersion::V2,
        _ => return Err(CaptureError::CaptureCapabilityNamespaceUnsupported.into()),
    };
    let record = load_declaration(snapshot, meta, &row, protocol, meter)?;
    match crate::lineage::read_admitted(snapshot, journal, meter)? {
        crate::lineage::RetainedIdentity::Present(identity)
            if &identity == record.store_identity() => {}
        _ => return Err(CaptureError::CaptureCapabilityForeignStore.into()),
    }
    Ok(Some(record))
}

fn graph_capture_present(row: Option<&[u8]>, has_attempts: bool) -> StoreResult<bool> {
    match row {
        Some(b"capture:v2:qualification") => Ok(true),
        None if !has_attempts => Ok(false),
        None => Err(CaptureError::CaptureCapabilityMissing.into()),
        Some(_) => Err(CaptureError::CaptureProtocolUnsupported.into()),
    }
}

fn declaration_format_unsupported(format: &StoreFormat) -> bool {
    format.schema_version != Some(2)
        || format.key_format != Some(1)
        || format.migration_target.is_some()
}

fn load_declaration(
    snapshot: &Snapshot,
    meta: &Keyspace,
    row: &[u8],
    protocol: CaptureProtocolVersion,
    meter: &mut super::admission::ReadMeter<'_>,
) -> StoreResult<CaptureQualification> {
    let length = snapshot
        .size_of(meta, row)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(CaptureError::CaptureCapabilityMissing)?;
    if length > 65_536 {
        return Err(CaptureError::RecordTooLarge.into());
    }
    meter.value(
        row.len(),
        usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?,
    )?;
    let raw = snapshot
        .get(meta, row)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(CaptureError::CaptureCapabilityMissing)?;
    meter.decode(&raw)?;
    let record = match protocol {
        CaptureProtocolVersion::V1 => serde_json::from_slice(&raw).map(CaptureQualification::V1),
        CaptureProtocolVersion::V2 => serde_json::from_slice(&raw).map(CaptureQualification::V2),
    }
    .map_err(|source| StoreError::Decode {
        key: protocol.qualification_key().to_string(),
        source,
    })?;
    Ok(record)
}

impl Store {
    pub fn require_legacy_structured_capture(&self) -> StoreResult<()> {
        let record = declaration(
            &self.db.snapshot(),
            &self.meta,
            &self.journal,
            &self.store_format()?,
        )?;
        require_legacy(&record)
    }
}

pub(crate) fn preflight_keyspaces(db: &Database) -> StoreResult<()> {
    let snapshot = db.snapshot();
    let mut relevant = false;
    for (name, prefix) in [
        (crate::META, "capture:"),
        (crate::META, "public_resource_capture"),
        (crate::META, "public_resource_frontier"),
        (crate::META, "public_resource_child"),
        (crate::META, "public_resource_school"),
        (crate::JOURNAL, "source_acquisition_attempt_"),
        (crate::JOURNAL, "public_resource_capture"),
        (crate::JOURNAL, "public_resource_frontier"),
        (crate::JOURNAL, "public_resource_child"),
        (crate::JOURNAL, "public_resource_school"),
        (crate::RECEIPTS, "source_acquisition_attempt_"),
        (crate::RECEIPTS, "public_resource_capture"),
        (crate::RECEIPTS, "public_resource_frontier"),
        (crate::RECEIPTS, "public_resource_child"),
        (crate::RECEIPTS, "public_resource_school"),
    ] {
        if !db.keyspace_exists(name) {
            continue;
        }
        let keyspace = db
            .keyspace(name, fjall::KeyspaceCreateOptions::default)
            .map_err(|source| StoreError::Open { source })?;
        relevant |= snapshot
            .prefix(&keyspace, prefix.as_bytes())
            .next()
            .is_some();
        if name == crate::META {
            let size = snapshot
                .size_of(&keyspace, crate::format::LIFECYCLE_REVISION_KEY)
                .map_err(|source| StoreError::Read { source })?;
            if size.is_some_and(|size| size <= 32) {
                let raw = snapshot
                    .get(&keyspace, crate::format::LIFECYCLE_REVISION_KEY)
                    .map_err(|source| StoreError::Read { source })?;
                relevant |= raw
                    .as_ref()
                    .and_then(|v| std::str::from_utf8(v).ok())
                    .and_then(|v| v.trim().parse::<u32>().ok())
                    == Some(CAPTURE_QUALIFICATION_LIFECYCLE);
            }
        }
    }
    if relevant {
        crate::format::existing::require_keyspaces(db)?;
    }
    Ok(())
}
