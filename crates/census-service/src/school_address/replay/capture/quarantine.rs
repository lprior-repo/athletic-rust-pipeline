use std::path::Path;

use super::atomic::{self, InstallOutcome};
use super::{hex, identical, io, load, refused, ReplayError, MAX_CAPTURE_BYTES};

const QUARANTINE_ATTEMPTS: u32 = 32;

pub(super) fn replace_with_evidence_sync<S, F>(
    path: &Path,
    tmp: &Path,
    bytes: &[u8],
    sync: &S,
    sync_file: &F,
) -> Result<(), ReplayError>
where
    S: Fn(&Path) -> Result<(), ReplayError>,
    F: Fn(&Path) -> Result<(), ReplayError>,
{
    let observed = load(path, MAX_CAPTURE_BYTES)?;
    retain_evidence(path, &observed, bytes, sync, sync_file)?;
    if !identical(path, &observed)? {
        return Err(refused(path, "capture changed during quarantine"));
    }
    std::fs::remove_file(path).map_err(|source| io(path, source))?;
    sync(path)?;
    match std::fs::hard_link(tmp, path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
            if identical(path, bytes)? {
                sync_file(path)
            } else {
                Err(refused(path, "capture destination changed during recovery"))
            }
        }
        Err(source) => Err(io(path, source)),
    }
}

fn retain_evidence<S, F>(
    path: &Path,
    observed: &[u8],
    expected: &[u8],
    sync: &S,
    sync_file: &F,
) -> Result<(), ReplayError>
where
    S: Fn(&Path) -> Result<(), ReplayError>,
    F: Fn(&Path) -> Result<(), ReplayError>,
{
    let digest = hex(observed);
    (0..QUARANTINE_ATTEMPTS)
        .map(|attempt| quarantine_label(&digest, attempt))
        .find_map(
            |label| match retain_one(path, observed, expected, &label, sync_file) {
                Ok(InstallOutcome::Retained) => Some(sync(path)),
                Ok(InstallOutcome::Occupied) => None,
                Err(error) => Some(Err(error)),
            },
        )
        .ok_or_else(|| refused(path, "quarantine evidence destinations are occupied"))?
}

fn quarantine_label(digest: &str, attempt: u32) -> String {
    match attempt {
        0 => format!("recovered-{digest}"),
        value => format!("recovered-{digest}-{value}"),
    }
}

fn retain_one<F>(
    path: &Path,
    observed: &[u8],
    expected: &[u8],
    label: &str,
    sync_file: &F,
) -> Result<InstallOutcome, ReplayError>
where
    F: Fn(&Path) -> Result<(), ReplayError>,
{
    let quarantine = path.with_extension(label);
    match link_evidence(path, &quarantine, observed, sync_file)? {
        InstallOutcome::Occupied => return Ok(InstallOutcome::Occupied),
        InstallOutcome::Retained => {}
    }
    let sidecar = path.with_extension(format!("{label}.sidecar"));
    let content = sidecar_content(&quarantine, observed, expected);
    atomic::install_new(&sidecar, content.as_bytes(), sync_file)
}

fn link_evidence<F>(
    path: &Path,
    quarantine: &Path,
    observed: &[u8],
    sync_file: &F,
) -> Result<InstallOutcome, ReplayError>
where
    F: Fn(&Path) -> Result<(), ReplayError>,
{
    match std::fs::hard_link(path, quarantine) {
        Ok(()) => {
            if identical(quarantine, observed)? {
                sync_file(quarantine)?;
                Ok(InstallOutcome::Retained)
            } else {
                Err(refused(
                    quarantine,
                    "capture changed while evidence was linked",
                ))
            }
        }
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
            if identical(quarantine, observed)? {
                sync_file(quarantine)?;
                Ok(InstallOutcome::Retained)
            } else {
                Ok(InstallOutcome::Occupied)
            }
        }
        Err(source) => Err(io(quarantine, source)),
    }
}

fn sidecar_content(quarantine: &Path, observed: &[u8], expected: &[u8]) -> String {
    format!(
        "quarantine: {}\nexpected_len: {}\nexpected_digest: {}\nobserved_len: {}\nobserved_digest: {}\nreason: bytes differ after install\n",
        quarantine.display(),
        expected.len(),
        hex(expected),
        observed.len(),
        hex(observed)
    )
}
