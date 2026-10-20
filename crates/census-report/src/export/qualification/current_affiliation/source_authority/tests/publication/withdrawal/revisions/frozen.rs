use super::*;
use crate::export::DatasetLineage;
use crate::report::Scope;
use std::path::{Path, PathBuf};

pub(super) struct FrozenRevision {
    pub(super) path: PathBuf,
    pub(super) digest: String,
    pub(super) lineage: DatasetLineage,
    pub(super) witness: Value,
    pub(super) workbook: PathBuf,
    pub(super) generation_digest: String,
    pub(super) rows: Value,
}

pub(super) fn freeze_and_check(
    fixture: &fixture::Fixture,
    dataset: &ExportDataset,
    frozen_path: &Path,
    output: &Path,
    expected: &Expected,
) -> TestResult<FrozenRevision> {
    let witness = witness(dataset, expected)?;
    dataset.save_frozen(frozen_path)?;
    let digest = dataset.archive_digest()?;
    inspect_wire(frozen_path)?;
    let workbook = publish(fixture, dataset, output)?;
    let rows = artifacts::readback(&workbook, dataset, expected)?;
    let verified = published::verify_published(&workbook)?;
    check!(eq; verified.workbook.mapped_athletes, u64::try_from(expected.selected_count())?);
    Ok(FrozenRevision {
        path: frozen_path.to_owned(),
        digest,
        lineage: dataset.lineage.clone(),
        witness,
        workbook,
        generation_digest: verified.generation_digest,
        rows,
    })
}

pub(super) fn reopen_and_check(revision: &FrozenRevision, expected: &Expected) -> TestResult {
    let reopened = ExportDataset::reopen_frozen_with_digest(&revision.path, &revision.digest)?;
    check!(eq; reopened.archive_digest()?, revision.digest);
    check!(eq; reopened.lineage, revision.lineage);
    check!(eq; witness(&reopened, expected)?, revision.witness);
    check!(eq; artifacts::readback(&revision.workbook, &reopened, expected)?, revision.rows);
    let publication = published::open_publication(
        &revision.workbook,
        &revision.lineage,
        Scope::AllSources,
        GradYear::CO2027,
    )?;
    check!(eq; witness(&publication.dataset, expected)?, revision.witness);
    check!(eq; artifacts::readback(&revision.workbook, &publication.dataset, expected)?, revision.rows);
    check!(eq; published::verify_published(&revision.workbook)?.generation_digest,
        revision.generation_digest);
    Ok(())
}
