use super::super::invariant;
use crate::report::file_digest::{hash_reader, FileDigestError};
use crate::report::{io_error, ReportResult};
use serde::{Deserialize, Serialize};
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
    let digest = hash_reader(file, MAX_ARTIFACT_BYTES).map_err(|error| match error {
        FileDigestError::Io(source) => io_error(path, source),
        FileDigestError::CountConversion(source) => invariant(source.to_string()),
        FileDigestError::InvalidReadLength => invariant("invalid file read length".into()),
        FileDigestError::CounterExhausted => invariant("artifact byte counter exhausted".into()),
        FileDigestError::BudgetExceeded => invariant("artifact grew past its byte budget".into()),
    })?;
    Ok(Artifact {
        bytes: digest.bytes,
        sha256: digest.sha256,
    })
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
        2..=8 | 10..=14 => names.extend(
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
