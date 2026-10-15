use super::{invariant, verify, ReportResult};
use std::path::{Path, PathBuf};

pub(in crate::workbook) struct ComparisonInput {
    directory: PathBuf,
    generation: String,
    bytes: u64,
    digest: String,
    rows: usize,
}

impl ComparisonInput {
    pub(in crate::workbook) fn athletes(&self) -> PathBuf {
        self.directory.join("athletes.csv")
    }

    pub(in crate::workbook) fn generation(&self) -> &str {
        &self.generation
    }

    pub(in crate::workbook) fn binding(&self) -> (u64, &str, usize) {
        (self.bytes, &self.digest, self.rows)
    }
}

pub(in crate::workbook) fn comparison_input(
    path: &Path,
    year: i16,
) -> ReportResult<ComparisonInput> {
    let path = std::fs::canonicalize(path).map_err(|error| crate::report::io_error(path, error))?;
    let (directory, manifest) = verify::preflight(&path)?;
    let options = manifest.selection.options()?;
    if options.grad_year != Some(year) || options.limit.is_some() {
        return Err(invariant(
            "reference comparison requires an unlimited matching graduation-year publication"
                .into(),
        ));
    }
    let dataset = verify::decode(&directory, &manifest)?;
    verify::verify_against(&directory, &manifest, &dataset)?;
    let artifact = manifest
        .artifacts
        .get("athletes.csv")
        .ok_or_else(|| invariant("reference comparison requires normalized athletes.csv".into()))?;
    let rows = manifest
        .normalized_csv_rows
        .as_ref()
        .and_then(|rows| rows.get("athletes.csv"))
        .ok_or_else(|| {
            invariant(
                "reference comparison requires a retained normalized row-count inventory".into(),
            )
        })?;
    Ok(ComparisonInput {
        directory,
        generation: manifest.generation_digest,
        bytes: artifact.bytes,
        digest: artifact.sha256.clone(),
        rows: usize::try_from(*rows)
            .map_err(|_| invariant("normalized row count exceeds platform limit".into()))?,
    })
}
