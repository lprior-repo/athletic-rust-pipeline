use super::archive_io::{
    create_directory, invalid, io_error, new_directory, present, sync_directory,
};
use super::FetchError;
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Damage {
    IncompletePair,
    MetadataLimitOrChanged,
    InvalidMetadata,
    BodySizeOrChanged,
    BodyIntegrity,
}

#[derive(Serialize)]
struct QuarantineReason<'a> {
    version: u8,
    body_path: &'a Path,
    meta_path: &'a Path,
    body_present: bool,
    meta_present: bool,
    reason: Damage,
}

pub(super) fn quarantine_pair(
    stage: &Path,
    body_path: &Path,
    meta_path: &Path,
    presence: (bool, bool),
    reason: Damage,
) -> Result<(), FetchError> {
    let evidence = begin_quarantine(stage, body_path, meta_path, presence, reason)?;
    if presence.0 {
        move_evidence(body_path, &evidence.join("body"))?;
    }
    if presence.1 {
        move_evidence(meta_path, &evidence.join("meta.json"))?;
    }
    Ok(())
}

pub(super) fn begin_quarantine(
    stage: &Path,
    body_path: &Path,
    meta_path: &Path,
    presence: (bool, bool),
    reason: Damage,
) -> Result<PathBuf, FetchError> {
    let root = stage
        .parent()
        .ok_or_else(|| invalid(stage, "quarantine has no cache root"))?;
    let quarantine = root.join("quarantine");
    create_directory(&quarantine)?;
    let evidence = new_directory(&quarantine, "capture")?;
    let record = QuarantineReason {
        version: 1,
        body_path,
        meta_path,
        body_present: presence.0,
        meta_present: presence.1,
        reason,
    };
    let staged_reason = stage.join("quarantine.reason.json");
    write_reason(&staged_reason, &record)?;
    let reason_path = evidence.join("reason.json");
    fs::rename(&staged_reason, &reason_path).map_err(|error| io_error(&reason_path, error))?;
    sync_directory(&evidence)?;
    Ok(evidence)
}

fn write_reason(path: &Path, reason: &QuarantineReason<'_>) -> Result<(), FetchError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| io_error(path, error))?;
    serde_json::to_writer_pretty(&mut file, reason).map_err(|source| FetchError::Encode {
        target: path.display().to_string(),
        source,
    })?;
    file.sync_all().map_err(|error| io_error(path, error))
}

pub(super) fn move_evidence(source: &Path, destination: &Path) -> Result<(), FetchError> {
    if present(destination)? {
        return Err(invalid(
            destination,
            "quarantine destination already exists",
        ));
    }
    File::open(source)
        .and_then(|file| file.sync_all())
        .map_err(|error| io_error(source, error))?;
    fs::rename(source, destination).map_err(|error| io_error(source, error))?;
    let evidence = destination
        .parent()
        .ok_or_else(|| invalid(destination, "evidence has no parent"))?;
    sync_directory(evidence)?;
    let root = source
        .parent()
        .ok_or_else(|| invalid(source, "cache has no parent"))?;
    sync_directory(root)
}
