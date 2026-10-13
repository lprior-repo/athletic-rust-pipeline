use super::*;
use crate::bests::{self, SharedSelection};
use crate::export::{freeze_v2, to_export_dataset, DatasetLineage, FrozenRoot};
use crate::report::{coverage_report, Scope};
use calamine::Data;
use census_domain::model::{GradYear, SchoolYear};
use serde_json::Value;

mod fixture;
mod readback;

use fixture::{Captured, Mutation, JOB};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type SelectedOptions = (Scope, Option<i16>, Option<usize>, SchoolYear);

struct Witness {
    lineage: DatasetLineage,
    evidence: Value,
    report: Value,
    bests: Vec<SharedSelection>,
    cells: Vec<Vec<Data>>,
    selection: SelectedOptions,
    pointer: PathBuf,
    archive: Vec<u8>,
    published_archive: Vec<u8>,
    generation: String,
}

impl Witness {
    fn capture(captured: &Captured) -> TestResult<Self> {
        readback::assert_baseline(&captured.dataset, &captured.workbook)?;
        let verified = verify_published(&captured.workbook)?;
        check!(eq; verified.workbook.mapped_athletes, 1);
        check!(eq; captured.dataset.lineage.schema_revision, 3);
        check!(eq; captured.dataset.lineage.policy_revision, 15);
        let selection = readback::selection(&captured.workbook)?;
        check!(eq; selection, selected_options(&captured.options));
        Ok(Self {
            lineage: captured.dataset.lineage.clone(),
            evidence: serde_json::to_value(&captured.dataset.coverage_evidence)?,
            report: serde_json::to_value(coverage_report(&captured.dataset, Some(2027))?)?,
            bests: selected_bests(&captured.dataset)?,
            cells: readback::coverage_cells(&captured.workbook)?,
            selection,
            pointer: std::fs::read_link(captured.store.out_dir().join("publication/current"))?,
            archive: std::fs::read(&captured.archive)?,
            published_archive: std::fs::read(readback::archive_path(&captured.workbook)?)?,
            generation: verified.generation_digest,
        })
    }

    fn dataset_unchanged(&self, dataset: &ExportDataset) -> TestResult {
        check!(eq; dataset.lineage, self.lineage);
        check!(eq; serde_json::to_value(&dataset.coverage_evidence)?, self.evidence);
        check!(eq; serde_json::to_value(coverage_report(dataset, Some(2027))?)?, self.report);
        check!(eq; selected_bests(dataset)?, self.bests);
        Ok(())
    }

    fn publication_unchanged(&self, store: &Store, workbook: &Path, archive: &Path) -> TestResult {
        let root = store.out_dir().join("publication");
        check!(eq; std::fs::read_link(root.join("current"))?, self.pointer);
        check!(eq; current_workbook(&root)?.as_path(), workbook);
        check!(eq; std::fs::read(archive)?, self.archive);
        check!(eq; std::fs::read(readback::archive_path(workbook)?)?, self.published_archive);
        check!(eq; readback::coverage_cells(workbook)?, self.cells);
        check!(eq; readback::selection(workbook)?, self.selection);
        check!(eq; verify_published(workbook)?.generation_digest, self.generation);
        Ok(())
    }
}

fn selected_options(options: &Options) -> SelectedOptions {
    (
        options.scope,
        options.grad_year,
        options.limit,
        options.school_year,
    )
}

fn selected_bests(dataset: &ExportDataset) -> crate::report::ReportResult<Vec<SharedSelection>> {
    bests::build_from_dataset(
        dataset,
        &bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
        },
    )
}

#[test]
fn published_coverage_readback_uses_captured_evidence_after_live_mutation_and_reopen() -> TestResult
{
    let captured = Captured::create()?;
    let witness = Witness::capture(&captured)?;
    fixture::mutate(&captured.store, Mutation::Both)?;
    readback::assert_live_changed(&captured.store, Mutation::Both)?;
    witness.dataset_unchanged(&captured.dataset)?;
    let Captured {
        directory,
        store,
        dataset,
        frozen,
        workbook,
        options,
        archive,
    } = captured;
    let root = store.root().to_path_buf();
    store.flush()?;
    drop(dataset);
    drop(store);
    let reopened = Store::open(&root)?;
    readback::assert_live_changed(&reopened, Mutation::Both)?;
    let archived = ExportDataset::reopen_frozen(&archive)?;
    witness.dataset_unchanged(&archived)?;
    let materialized = to_export_dataset(&frozen, &reopened)?;
    witness.dataset_unchanged(&materialized)?;
    let reused = freeze_v2(&reopened, JOB)?;
    assert_same_root(&frozen, &reused)?;
    witness.dataset_unchanged(&to_export_dataset(&reused, &reopened)?)?;
    witness.publication_unchanged(&reopened, &workbook, &archive)?;
    crate::workbook::verify::verify_frozen(&workbook, &archived, &options)?;
    let opened = open_publication(
        &workbook,
        &archived.lineage,
        Scope::AllSources,
        GradYear::CO2027,
    )?;
    witness.dataset_unchanged(&opened.dataset)?;
    check!(eq; opened.generation_digest, witness.generation);
    check!(directory.path().is_dir());
    Ok(())
}

fn assert_same_root(before: &FrozenRoot, after: &FrozenRoot) -> TestResult {
    check!(eq; after.manifest_digest(), before.manifest_digest());
    check!(eq; after.attempt(), before.attempt());
    check!(eq; after.manifest(), before.manifest());
    Ok(())
}

#[test]
fn coverage_only_change_refuses_stale_publication_and_retains_archives() -> TestResult {
    stale_input_refused(Mutation::CoverageOnly)
}

#[test]
fn coach_lanes_only_change_refuses_stale_publication_and_retains_archives() -> TestResult {
    stale_input_refused(Mutation::CoachLanesOnly)
}

fn stale_input_refused(mutation: Mutation) -> TestResult {
    let captured = Captured::create()?;
    let witness = Witness::capture(&captured)?;
    fixture::mutate(&captured.store, mutation)?;
    readback::assert_live_changed(&captured.store, mutation)?;
    let live = ExportDataset::load(&captured.store)?;
    check!(ne; live.lineage.source_digest, captured.dataset.lineage.source_digest);
    check!(ne; live.lineage.input_digest, captured.dataset.lineage.input_digest);
    match mutation {
        Mutation::CoverageOnly => {
            check!(eq; live.coverage_evidence.coach_lanes, captured.dataset.coverage_evidence.coach_lanes)
        }
        Mutation::CoachLanesOnly => {
            check!(eq; live.coverage_evidence.rows, captured.dataset.coverage_evidence.rows)
        }
        Mutation::Both => return Err("stale-input tests require one changed table".into()),
    }
    check!(matches!(
        captured.dataset.ensure_snapshot(&captured.store.snapshot()),
        Err(ReportError::Invariant { ref detail }) if detail.contains("stale or foreign")
    ));
    check!(matches!(
        ExportDataset::for_job(&captured.store, JOB),
        Err(ReportError::Invariant { ref detail }) if detail.contains("stale or foreign")
    ));
    let censuses = crate::workbook::Censuses::of(&captured.dataset, &captured.store.out_dir());
    check!(matches!(
        crate::workbook::build_from(
            &captured.dataset,
            &captured.store,
            &captured.options,
            &censuses
        ),
        Err(ReportError::Publication(
            PublicationError::InputFenceRefused { .. }
        ))
    ));
    witness.dataset_unchanged(&captured.dataset)?;
    witness.publication_unchanged(&captured.store, &captured.workbook, &captured.archive)?;
    Ok(())
}
