use super::{CanonicalPerformance, ExportDataset, TestResult};
use census_domain::model::GradYear;
use census_report::workbook::publication::{self, VerifiedPublication};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct RetainedGeneration {
    pub path: String,
    workbook: Vec<u8>,
    manifest: Vec<u8>,
}

impl RetainedGeneration {
    pub fn capture(path: String) -> TestResult<Self> {
        let generation = Path::new(&path).parent().ok_or("retained generation")?;
        let workbook = std::fs::read(&path)?;
        let manifest = std::fs::read(generation.join("manifest.json"))?;
        Ok(Self {
            path,
            workbook,
            manifest,
        })
    }

    pub fn directory(&self) -> TestResult<&Path> {
        Path::new(&self.path)
            .parent()
            .ok_or_else(|| "retained generation".into())
    }

    pub fn preserved(&self) -> TestResult {
        check!(eq; std::fs::read(&self.path)?, self.workbook);
        check!(eq; std::fs::read(self.directory()?.join("manifest.json"))?, self.manifest);
        Ok(())
    }
}

pub(super) fn verify(
    path: &Path,
    scope: census_report::report::Scope,
    performances: &[CanonicalPerformance; 2],
) -> TestResult<VerifiedPublication> {
    let verified = publication::verify_published(path)?;
    check!(eq; verified.workbook.mapped_athletes, 1);
    let generation = path.parent().ok_or("service generation")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    let archive_digest = manifest
        .pointer("/artifacts/frozen-input.json/sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or("published frozen archive commitment")?;
    let frozen = ExportDataset::reopen_frozen_with_digest(
        &generation.join("frozen-input.json"),
        archive_digest,
    )?;
    assert_frozen(&frozen, performances)?;
    let lineage = frozen.lineage.clone();
    drop(frozen);
    let reopened = publication::open_publication(path, &lineage, scope, GradYear::CO2027)?;
    check!(eq; reopened.generation_digest, verified.generation_digest);
    check!(eq; reopened.workbook.mapped_athletes, 1);
    assert_frozen(&reopened.dataset, performances)?;
    Ok(verified)
}

fn assert_frozen(dataset: &ExportDataset, performances: &[CanonicalPerformance; 2]) -> TestResult {
    let retained: BTreeSet<_> = dataset.performances.iter().map(|row| &row.id).collect();
    let expected: BTreeSet<_> = performances.iter().map(|row| &row.id).collect();
    check!(eq; dataset.performances.len(), 2);
    check!(eq; retained, expected);
    Ok(())
}
