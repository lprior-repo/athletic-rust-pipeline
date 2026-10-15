use super::super::invariant;
use crate::report::{io_error, ReportResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

const MAX_ARTIFACT_BYTES: u64 = crate::export::MAX_FROZEN_INPUT_BYTES;
pub(super) const MAX_BUNDLE_BYTES: u64 = 256 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Artifact {
    pub(super) bytes: u64,
    pub(super) sha256: String,
}

pub(super) fn hash_artifact(path: &Path) -> ReportResult<Artifact> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_ARTIFACT_BYTES {
        return Err(invariant(format!(
            "invalid or oversized generation artifact: {}",
            path.display()
        )));
    }
    let mut file = std::fs::File::open(path).map_err(|source| io_error(path, source))?;
    file.sync_all().map_err(|source| io_error(path, source))?;
    let artifact = hash_file(&mut file, path)?;
    if artifact.bytes != metadata.len() {
        return Err(invariant("artifact changed while hashing".to_string()));
    }
    Ok(artifact)
}

fn hash_file(file: &mut std::fs::File, path: &Path) -> ReportResult<Artifact> {
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65_536];
    let mut total = 0_u64;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|source| io_error(path, source))?;
        if read == 0 {
            break;
        }
        let chunk = buffer
            .get(..read)
            .ok_or_else(|| invariant("invalid file read length".to_string()))?;
        digest.update(chunk);
        total = accumulate_bytes(total, read)?;
    }
    Ok(Artifact {
        bytes: total,
        sha256: format!("{:x}", digest.finalize()),
    })
}

fn accumulate_bytes(total: u64, read: usize) -> ReportResult<u64> {
    let total = total
        .checked_add(u64::try_from(read).map_err(|error| invariant(error.to_string()))?)
        .ok_or_else(|| invariant("artifact byte counter exhausted".to_string()))?;
    if total > MAX_ARTIFACT_BYTES {
        return Err(invariant("artifact grew past its byte budget".to_string()));
    }
    Ok(total)
}

pub(super) fn artifact_names(revision: u32, grad_year: Option<i16>) -> ReportResult<Vec<String>> {
    let cohort = match grad_year.map(|year| format!("co{year}")) {
        Some(value) => value,
        None => "all".to_string(),
    };
    let mut names = vec![
        "audit.json".to_string(),
        format!("best-results-{cohort}.csv"),
        format!("best-results-{cohort}.jsonl"),
        "census-all-sources.json".to_string(),
        "census-core.json".to_string(),
        "frozen-input.json".to_string(),
        "recruiting.csv".to_string(),
        "school-contacts.csv".to_string(),
        "contact-research.csv".to_string(),
        "workbook.xlsx".to_string(),
    ];
    match revision {
        1 => {}
        2..=6 => names.extend(
            [
                "event-catalog.json",
                "relay-dispositions.json",
                "relays.jsonl",
            ]
            .map(str::to_string),
        ),
        _ => {
            return Err(invariant(
                "unsupported publication artifact revision".into(),
            ))
        }
    }
    names.extend(
        super::super::inventory::names(revision)?
            .iter()
            .map(|name| (*name).to_owned()),
    );
    names.sort_unstable();
    Ok(names)
}
