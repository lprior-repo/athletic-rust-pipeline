use super::input::Inputs;
use super::{defect, diff, read};
use crate::report::{RelayProjection, ReportError, ReportResult};
use std::path::Path;

const MAX_BYTES: u64 = 128 * 1024 * 1024;
const LIMITS: read::Limits = read::Limits {
    bytes: MAX_BYTES,
    records: 1_000_000,
};

pub(super) fn verify(directory: &Path, inputs: &Inputs<'_>) -> ReportResult<()> {
    verify_json(
        &directory.join("event-catalog.json"),
        &inputs.admission.audit(),
    )?;
    let relays = RelayProjection::of(&inputs.derivation, &inputs.admission)?;
    verify_json(
        &directory.join("relay-dispositions.json"),
        &relays.admissions,
    )?;
    let path = directory.join("relays.jsonl");
    let mut position = 0usize;
    let total = read::lines(&path, LIMITS, "published relays", |line, text| {
        let expected = relays.selections.get(position).ok_or_else(|| {
            defect(format!(
                "{} line {line} is an unexpected accepted relay",
                path.display()
            ))
        })?;
        let found = serde_json::from_str(text).map_err(|source| ReportError::Decode {
            path: path.clone(),
            line,
            source,
        })?;
        compare(&path, &encoded(expected)?, &found)?;
        position = position
            .checked_add(1)
            .ok_or_else(|| defect("relay readback count overflow".into()))?;
        Ok(())
    })?;
    if total != relays.selections.len() {
        return Err(defect(format!(
            "{} lost expected accepted relay rows",
            path.display()
        )));
    }
    Ok(())
}

fn verify_json(path: &Path, expected: &impl serde::Serialize) -> ReportResult<()> {
    let found = read::json_value(path, MAX_BYTES, "recruiting catalog disposition audit")?;
    compare(path, &encoded(expected)?, &found)
}

fn encoded(value: &impl serde::Serialize) -> ReportResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|source| ReportError::Invariant {
        detail: format!("encoding independent catalog expectation: {source}"),
    })
}

fn compare(
    path: &Path,
    expected: &serde_json::Value,
    found: &serde_json::Value,
) -> ReportResult<()> {
    if expected != found {
        return Err(defect(format!(
            "{} differs from retained evidence: {}",
            path.display(),
            diff::difference(expected, found)
        )));
    }
    Ok(())
}
