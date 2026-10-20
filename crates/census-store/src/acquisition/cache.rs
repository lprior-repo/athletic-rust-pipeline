use super::{read, AcquisitionReadBudget, CaptureError};
use crate::{StoreError, StoreResult};
use census_domain::model::*;
use fjall::{Keyspace, Snapshot};
use serde::Deserialize;
use std::io::Read;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EligibleEnvelope {
    version: VersionTwo,
    metadata: serde_json::Value,
    original_occurrence: AcquisitionOccurrenceRefV1,
}
#[derive(Deserialize)]
#[serde(try_from = "u8")]
struct VersionTwo;
impl TryFrom<u8> for VersionTwo {
    type Error = CaptureError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 2 {
            Ok(Self)
        } else {
            Err(CaptureError::CacheEnvelopeUnsupported)
        }
    }
}
impl From<VersionTwo> for u8 {
    fn from(_: VersionTwo) -> Self {
        2
    }
}

pub(super) fn inventory(
    root: &Path,
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &CaptureQualification,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let http = root.join("http");
    let entries = match std::fs::read_dir(&http) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(io(&http, source)),
    };
    for (position, entry) in entries.take(1_048_577).enumerate() {
        if position == 1_048_576 {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "eligible cache inventory",
                required: 1_048_577,
                limit: 1_048_576,
            }
            .into());
        }
        let entry = entry.map_err(|source| io(&http, source))?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or(CaptureError::CacheEnvelopeUnsupported)?;
        if !name.ends_with(".meta.json") {
            continue;
        }
        process_eligible_entry(
            root,
            snapshot,
            journal,
            receipts,
            declaration,
            &http,
            &entry.path(),
            name,
            budget,
        )?;
    }
    Ok(())
}
fn process_eligible_entry(
    root: &Path,
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &CaptureQualification,
    http: &Path,
    path: &Path,
    name: &str,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let raw = read_metadata(path, budget)?;
    let Some(envelope) = parse_eligible(&raw, name)? else {
        return Ok(());
    };
    if envelope.original_occurrence.attempt().store_identity() != declaration.store_identity() {
        return Err(CaptureError::CaptureCapabilityForeignStore.into());
    }
    let facts = read::read_work(
        snapshot,
        journal,
        receipts,
        envelope.original_occurrence.attempt().work_id(),
        budget,
        declaration.protocol(),
    )?;
    let verified =
        read::reusable_body_reference(root, &facts, &envelope.original_occurrence, budget)?;
    match_metadata(envelope.metadata, verified.prepared())?;
    let key = name
        .strip_suffix(".meta.json")
        .ok_or(CaptureError::CacheEnvelopeUnsupported)?;
    super::backing::verify_file(
        &http.join(format!("{key}.body")),
        verified.prepared().content_bytes(),
        verified.prepared().content_digest(),
        budget,
    )
}

fn parse_eligible(raw: &[u8], name: &str) -> StoreResult<Option<EligibleEnvelope>> {
    let probe: serde_json::Value =
        serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
            key: name.to_string(),
            source,
        })?;
    if !is_envelope(&probe) {
        return Ok(None);
    }
    let envelope: EligibleEnvelope =
        serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
            key: name.to_string(),
            source,
        })?;
    let VersionTwo = envelope.version;
    Ok(Some(envelope))
}
fn match_metadata(
    mut metadata: serde_json::Value,
    prepared: &PreparedCaptureInputV1,
) -> StoreResult<()> {
    if !metadata.is_object() {
        return Err(CaptureError::CacheEnvelopeUnsupported.into());
    }
    metadata.sort_all_objects();
    let canonical = serde_json::to_vec_pretty(&metadata).map_err(|source| StoreError::Json {
        detail: "checking eligible metadata identity".to_string(),
        source,
    })?;
    if canonical != prepared.metadata_bytes() {
        return Err(CaptureError::CacheOccurrenceMismatch.into());
    }
    Ok(())
}

pub(crate) fn envelope_digest(raw: &[u8]) -> StoreResult<Option<String>> {
    let probe: serde_json::Value =
        serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
            key: "eligible metadata".to_string(),
            source,
        })?;
    if !is_envelope(&probe) {
        return Ok(None);
    }
    let envelope: EligibleEnvelope =
        serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
            key: "eligible metadata envelope".to_string(),
            source,
        })?;
    let VersionTwo = envelope.version;
    let digest = envelope
        .metadata
        .get("content_digest")
        .and_then(|value| value.as_str())
        .ok_or(CaptureError::CacheEnvelopeUnsupported)?;
    AcquisitionDigest::parse(digest).map_err(CaptureError::Domain)?;
    Ok(Some(digest.to_string()))
}
fn is_envelope(value: &serde_json::Value) -> bool {
    value.get("version").is_some()
        || value.get("metadata").is_some()
        || value.get("original_occurrence").is_some()
}
fn read_metadata(path: &Path, budget: &mut AcquisitionReadBudget) -> StoreResult<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io(path, source))?;
    if !metadata.is_file() || metadata.len() > 65_536 {
        return Err(CaptureError::RecordTooLarge.into());
    }
    let length = usize::try_from(metadata.len()).map_err(|_| StoreError::CounterOverflow)?;
    budget.encoded(length)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length).map_err(|source| {
        CaptureError::allocation_reservation_failed(
            "eligible metadata buffer",
            Some(length),
            source,
        )
    })?;
    let mut file = std::fs::File::open(path).map_err(|source| io(path, source))?;
    (&mut file)
        .take(metadata.len())
        .read_to_end(&mut bytes)
        .map_err(|source| io(path, source))?;
    let mut extra = [0_u8; 1];
    if bytes.len() != length || file.read(&mut extra).map_err(|source| io(path, source))? != 0 {
        return Err(CaptureError::BackingIntegrity.into());
    }
    Ok(bytes)
}
fn io(path: &Path, source: std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}
