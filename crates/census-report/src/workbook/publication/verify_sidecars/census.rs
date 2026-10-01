use crate::report::{ReportError, ReportResult};
use crate::workbook::Censuses;
use serde::Serialize;
use std::path::Path;

use super::input::Inputs;
use super::{defect, diff, read};

const MAX_BYTES: u64 = 64 * 1024 * 1024;

pub(super) fn verify(directory: &Path, inputs: &Inputs<'_>) -> ReportResult<()> {
    let reports = Censuses::of(inputs.dataset, &inputs.out_root);
    compare(directory, "census-core.json", &reports.core)?;
    compare(directory, "census-all-sources.json", &reports.all_sources)
}

fn compare(directory: &Path, name: &str, report: &impl Serialize) -> ReportResult<()> {
    let path = directory.join(name);
    let found = read::json_value(&path, MAX_BYTES, name)?;
    let expected = serde_json::to_value(report).map_err(|source| ReportError::Decode {
        path: path.clone(),
        line: 0,
        source,
    })?;
    if found == expected {
        return Ok(());
    }
    Err(defect(format!(
        "{name} does not match the shared census reduction: {}",
        diff::difference(&expected, &found)
    )))
}
