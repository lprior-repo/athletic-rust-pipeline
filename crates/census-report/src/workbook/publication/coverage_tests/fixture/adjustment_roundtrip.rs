use super::super::{fixture, readback, selected_bests, TestResult, Witness};
use super::{coverage_row, pending_lane, Captured, Mutation};
use crate::export::selected_test_fixture::{coverage_time_fixture, CoverageTimeFixture};
use crate::export::{ExportDataset, ExportJobRequest};
use crate::report::{ReportError, Scope};
use crate::workbook::publication::{open_publication, verify_sidecars};
use census_domain::model::{
    serialized_bytes, serialized_digest, Adjustment, CanonicalPerformance, ExactSeconds, GradYear,
    Mark, SourceRef,
};
use census_store::{Store, Table};

const JOB: &str = "publication:adjustment-coverage-roundtrip";
mod sidecars;
use sidecars::sidecar_forgeries_are_refused;

fn seconds(value: &str) -> TestResult<Mark> {
    Ok(Mark::TimeSeconds(ExactSeconds::parse(value)?))
}

fn adjustment(original: &str) -> TestResult<Adjustment> {
    Ok(Adjustment::track_size_conversion(
        Some(seconds(original)?),
        seconds("50.00")?,
        Some("Converted from 52.00 for Track Size.".into()),
        Some(SourceRef::new(
            "synthetic_track_conversion",
            Some("https://example.test/coverage".into()),
        )),
    ))
}
async fn physical_capture() -> TestResult<Captured> {
    let fixture =
        coverage_time_fixture(5200).map_err(|error| -> Box<dyn std::error::Error> { error })?;
    fixture.store.replace(Table::Coverage, &coverage_row(0))?;
    fixture.store.replace(Table::CoachLanes, &pending_lane()?)?;
    let request = ExportJobRequest::SelectedPublicationV1 {
        selection: fixture.selection.clone(),
    };
    let dataset = ExportDataset::for_job_request(&fixture.store, JOB, &request)?;
    let options = crate::workbook::options_for_selected_dataset(&dataset, None)?;
    let censuses =
        crate::workbook::Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    let workbook = crate::workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
    let archive = fixture.directory.path().join("adjustment-frozen.json");
    dataset.save_frozen(&archive)?;
    let digest = dataset.archive_digest()?;
    let CoverageTimeFixture {
        directory,
        store,
        selection,
        ..
    } = fixture;
    Ok(Captured {
        directory,
        store,
        selection,
        dataset,
        workbook,
        options,
        archive,
        digest,
    })
}

fn check_physical(dataset: &ExportDataset) -> TestResult {
    check!(eq; dataset.performances.len(), 3);
    let performance = dataset
        .performances
        .iter()
        .find(|p| p.date == "2026-05-15")
        .ok_or("captured May 15 performance missing")?;
    check!(eq; performance.mark, seconds("52.00")?);
    check!(performance.adjustment.is_none());
    check!(performance.retained_conflicts.is_empty());
    check!(eq; dataset.coverage_evidence.rows, vec![coverage_row(0)]);
    check!(eq; dataset.coverage_evidence.coach_lanes, vec![pending_lane()?]);
    let rows = selected_bests(dataset)?;
    check!(eq; rows.len(), 1);
    let selected = rows.first().ok_or("captured adjusted best missing")?;
    check!(eq; selected.result.value, 52_000_000_000);
    check!(eq; selected.result.normalized, Some(52.0));
    check!(eq; selected.result.mark, seconds("52.00")?);
    check!(selected.result.original_mark.is_none());
    check!(selected.result.adjustment_kind.is_none());
    check!(selected.result.adjustment_basis.is_none());
    check!(selected.result.adjustment_note.is_none());
    check!(selected.result.adjustment_source.is_none());
    check!(eq; selected.source.performance_id, performance.id);
    check!(selected.conflicts.is_empty());
    Ok(())
}

fn same_dataset(dataset: &ExportDataset, expected: &[u8], witness: &Witness) -> TestResult {
    check_physical(dataset)?;
    check!(eq; serialized_bytes(dataset)?, expected);
    check!(eq; serialized_digest(dataset)?, dataset.lineage.input_digest);
    witness.dataset_unchanged(dataset)
}

fn mutate_adjustment(captured: &Captured) -> TestResult {
    let mut performance = captured
        .dataset
        .performances
        .iter()
        .find(|p| p.date == "2026-05-15")
        .ok_or("May 15 physical performance for mutation missing")?
        .clone();
    performance.adjustment = Some(adjustment("52.00")?);
    captured.store.append(Table::Performances, &performance)?;
    performance.adjustment = Some(adjustment("53.00")?);
    captured.store.append(Table::Performances, &performance)?;
    let updated = captured
        .store
        .scan::<CanonicalPerformance>(Table::Performances)?
        .into_iter()
        .find(|p| p.date == "2026-05-15")
        .ok_or("updated May 15 performance missing")?;
    check!(!updated.retained_conflicts.is_empty());
    let live_objects = tempfile::tempdir()?;
    physical_refusal(ExportDataset::from_selected_snapshot(
        &captured.store,
        &captured.store.snapshot(),
        &captured.selection,
        live_objects.path(),
    ))?;
    Ok(())
}

#[tokio::test]
async fn physical_bests_and_nonempty_coverage_survive_json_chunks_publication_and_live_mutation(
) -> TestResult {
    let captured = physical_capture().await?;
    check_physical(&captured.dataset)?;
    let expected = serialized_bytes(&captured.dataset)?;
    let witness = Witness::capture_with_best(&captured, 52_000_000_000)?;
    let live = ExportDataset::for_job_request(
        &captured.store,
        JOB,
        &ExportJobRequest::SelectedPublicationV1 {
            selection: captured.selection.clone(),
        },
    )?;
    check_physical(&live)?;
    check!(eq; serialized_bytes(&live)?, expected);
    check!(eq; live.lineage.input_digest, captured.dataset.lineage.input_digest);
    drop(live);
    check!(captured.dataset.selected_profile_id().is_some());
    check!(eq; captured.digest, captured.dataset.archive_digest()?);
    same_dataset(
        &ExportDataset::reopen_frozen_with_digest(&captured.archive, &captured.digest)?,
        &expected,
        &witness,
    )?;
    sidecar_forgeries_are_refused(&captured)?;
    fixture::mutate(&captured.store, Mutation::Both)?;
    readback::assert_live_changed(&captured.store, &captured.selection, Mutation::Both)?;
    mutate_adjustment(&captured)?;
    same_dataset(&captured.dataset, &expected, &witness)?;
    check!(
        matches!(captured.dataset.ensure_snapshot(&captured.store.snapshot()),
        Err(ReportError::Invariant { ref detail }) if detail.contains("stale or foreign"))
    );
    cold_roundtrip(captured, &expected, &witness)
}

fn cold_roundtrip(captured: Captured, expected: &[u8], witness: &Witness) -> TestResult {
    let Captured {
        directory,
        store,
        selection,
        dataset,
        workbook,
        options,
        archive,
        digest,
    } = captured;
    let root = store.root().to_path_buf();
    store.flush()?;
    drop(dataset);
    drop(store);
    let (reopened, writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
    drop(writer);
    let live_objects = tempfile::tempdir()?;
    physical_refusal(ExportDataset::from_selected_snapshot(
        &reopened,
        &reopened.snapshot(),
        &selection,
        live_objects.path(),
    ))?;
    let archived = ExportDataset::reopen_frozen_with_digest(&archive, &digest)?;
    same_dataset(&archived, expected, witness)?;
    check!(matches!(
        ExportDataset::for_job_request(
            &reopened,
            JOB,
            &ExportJobRequest::SelectedPublicationV1 {
                selection: selection.clone(),
            }
        ),
        Err(ReportError::Invariant { ref detail }) if detail.contains("stale or foreign")
    ));
    same_dataset(
        &ExportDataset::reopen_frozen_with_digest(&archive, &digest)?,
        expected,
        witness,
    )?;
    witness.publication_unchanged(&reopened, &workbook, &archive)?;
    crate::workbook::verify::verify_frozen(&workbook, &archived, &options)?;
    let opened = open_publication(
        &workbook,
        &archived.lineage,
        Scope::AllSources,
        GradYear::CO2027,
    )?;
    same_dataset(&opened.dataset, expected, witness)?;
    check!(eq; opened.generation_digest, witness.generation);
    check!(directory.path().is_dir());
    Ok(())
}

fn physical_refusal(result: crate::report::ReportResult<ExportDataset>) -> TestResult {
    check!(matches!(result, Err(ReportError::Invariant { ref detail })
        if detail == "school-proved performance conflicts with a physical source"));
    Ok(())
}

#[test]
fn invented_track_size_adjustment_refuses_physical_source_equality() -> TestResult {
    let fixture =
        coverage_time_fixture(5200).map_err(|error| -> Box<dyn std::error::Error> { error })?;
    check!(eq; fixture.performance.mark, seconds("52.00")?);
    check!(fixture.performance.adjustment.is_none());
    let mut invented = fixture.performance.clone();
    invented.adjustment = Some(adjustment("52.00")?);
    fixture.store.append(Table::Performances, &invented)?;
    let before = fixture.store.snapshot().tables_digest(&Table::ALL)?;
    let receipts = fixture.store.receipt_count()?;
    let objects = tempfile::tempdir()?;
    physical_refusal(ExportDataset::from_selected_snapshot(
        &fixture.store,
        &fixture.store.snapshot(),
        &fixture.selection,
        objects.path(),
    ))?;
    check!(eq; fixture.store.snapshot().tables_digest(&Table::ALL)?, before);
    check!(eq; fixture.store.receipt_count()?, receipts);
    Ok(())
}
