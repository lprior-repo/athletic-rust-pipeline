use super::super::{invalid, objects};
use crate::report::{ReportError, ReportResult};
use std::path::Path;

pub(in crate::export::dataset::frozen) fn retain(
    directory: &Path,
    stage: &str,
    error: ReportError,
) -> ReportError {
    match record(directory, stage, &error) {
        Ok(()) => error,
        Err(cleanup) => ReportError::Cleanup {
            operation: Box::new(error),
            cleanup: Box::new(cleanup),
        },
    }
}

fn record(directory: &Path, stage: &str, error: &ReportError) -> ReportResult<()> {
    let refusals = directory.join("refusals");
    census_store::fs::create_dir_all_synced(&refusals)
        .map_err(|source| crate::report::io_error(&refusals, source))?;
    let bytes = serde_json::to_vec(&serde_json::json!({
        "codec": "census-freeze-refusal-v1",
        "stage": stage,
        "capture": directory.display().to_string(),
        "reason": error.to_string(),
    }))
    .map_err(|source| invalid(&format!("encoding freeze refusal: {source}")))?;
    if bytes.len() > 64 * 1024 {
        return Err(invalid("freeze refusal exceeds its metadata envelope"));
    }
    objects::install(&refusals, &bytes).map(|_| ())
}
