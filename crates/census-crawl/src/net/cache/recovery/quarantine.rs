use super::{codec, paths, shell, stage};
use crate::net::cache::quarantine::{Damage, QuarantineReason};
use crate::net::FetchError;
use census_store::RecoveryArchiveIo;
use std::path::Path;

pub(super) fn pair(
    stage_path: &Path,
    body: &Path,
    metadata: &Path,
    presence: (bool, bool),
    damage: Damage,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    let root = stage_path
        .parent()
        .ok_or_else(|| shell::limit("quarantine ancestry"))?;
    let record = QuarantineReason {
        version: 1,
        body_path: body,
        meta_path: metadata,
        body_present: presence.0,
        meta_present: presence.1,
        reason: damage,
    };
    let count =
        codec::writer::count(&record, io)?.ok_or_else(|| shell::limit("quarantine encoding"))?;
    let encoded = codec::writer::encode(&record, count, io)?;
    let quarantine = paths::child(root, "quarantine", io)?;
    paths::safe_directory(&quarantine, io)?;
    let evidence = stage::directory(&quarantine, "capture", io)?;
    let staged_reason = stage::write_leaf(stage_path, "quarantine.reason.json", &encoded, io)?;
    let reason = paths::child(&evidence, "reason.json", io)?;
    shell::rename(&staged_reason, &reason, io)?;
    shell::sync_directory(&evidence, io)?;
    if presence.0 {
        move_evidence(body, &evidence, "body", io)?;
    }
    if presence.1 {
        move_evidence(metadata, &evidence, "meta.json", io)?;
    }
    for path in [&quarantine, &evidence, &staged_reason, &reason] {
        io.release_owned(path.as_os_str().len())?;
    }
    io.release_owned(encoded.capacity())?;
    Ok(())
}

fn move_evidence(
    source: &Path,
    evidence: &Path,
    name: &str,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    let destination = paths::child(evidence, name, io)?;
    if shell::regular(&destination, io)?.is_some() {
        return Err(shell::invalid(
            &destination,
            "quarantine destination already exists",
            io,
        )?);
    }
    let file = shell::open(source, io)?;
    shell::sync(&file, source, io)?;
    shell::rename(source, &destination, io)?;
    shell::sync_directory(evidence, io)?;
    shell::sync_directory(
        source
            .parent()
            .ok_or_else(|| shell::limit("quarantine ancestry"))?,
        io,
    )?;
    io.release_owned(destination.as_os_str().len())?;
    Ok(())
}
