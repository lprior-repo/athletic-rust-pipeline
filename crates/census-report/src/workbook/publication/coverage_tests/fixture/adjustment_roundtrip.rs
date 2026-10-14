use super::super::{assert_same_root, fixture, readback, selected_bests, TestResult, Witness};
use super::{coverage_row, pending_lane, seed_population, Captured, Mutation};
use crate::export::{freeze_v2, to_export_dataset, ExportDataset};
use crate::report::{ReportError, Scope};
use crate::workbook::publication::{open_publication, verify_sidecars};
use census_domain::model::{
    serialized_bytes, serialized_digest, Adjustment, AdjustmentBasis, AdjustmentKind,
    CanonicalPerformance, ExactSeconds, GradYear, Mark, SourceRef,
};
use census_store::{Store, Table};
use serde_json::Value;
use std::path::Path;

const JOB: &str = "publication:adjustment-coverage-roundtrip";
const CSV: &str = "best-results-co2027.csv";
const JSONL: &str = "best-results-co2027.jsonl";

fn seconds(value: &str) -> TestResult<Mark> {
    Ok(Mark::TimeSeconds(ExactSeconds::parse(value)?))
}

fn adjustment(original: &str) -> TestResult<Adjustment> {
    Ok(Adjustment::track_size_conversion(
        Some(seconds(original)?),
        seconds("50.00")?,
        Some("Converted from 52.00 for Track Size.".into()),
        Some(SourceRef::new(
            "wiaa_results",
            Some("https://example.test/coverage".into()),
        )),
    ))
}

fn only_performance(store: &Store) -> TestResult<CanonicalPerformance> {
    let rows = store.scan::<CanonicalPerformance>(Table::Performances)?;
    check!(eq; rows.len(), 1);
    rows.into_iter()
        .next()
        .ok_or_else(|| "adjustment fixture performance missing".into())
}

fn adjusted_capture() -> TestResult<Captured> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    seed_population(&store)?;
    let mut performance = only_performance(&store)?;
    performance.adjustment = Some(adjustment("52.00")?);
    store.append(Table::Performances, &performance)?;
    check!(only_performance(&store)?.retained_conflicts.is_empty());
    store.replace(Table::Coverage, &coverage_row(0))?;
    store.replace(Table::CoachLanes, &pending_lane()?)?;
    let frozen = freeze_v2(&store, JOB)?;
    let dataset = ExportDataset::for_job(&store, JOB)?;
    let archive = store
        .out_dir()
        .join("export-inputs")
        .join(format!("{}.json", serialized_digest(&JOB)?));
    let options = crate::workbook::publication::fence_tests::options(None)?;
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir(), &[]);
    let workbook = crate::workbook::build_from(&dataset, &store, &options, &censuses)?;
    Ok(Captured {
        directory,
        store,
        dataset,
        frozen,
        workbook,
        options,
        archive,
    })
}

fn check_adjustment(dataset: &ExportDataset) -> TestResult {
    check!(eq; dataset.performances.len(), 1);
    let performance = dataset
        .performances
        .first()
        .ok_or("captured performance missing")?;
    check!(eq; performance.mark, seconds("50.00")?);
    check!(eq; performance.adjustment, Some(adjustment("52.00")?));
    check!(performance.retained_conflicts.is_empty());
    check!(eq; dataset.coverage_evidence.rows, vec![coverage_row(0)]);
    check!(eq; dataset.coverage_evidence.coach_lanes, vec![pending_lane()?]);
    let rows = selected_bests(dataset)?;
    check!(eq; rows.len(), 1);
    let selected = rows.first().ok_or("captured adjusted best missing")?;
    check!(eq; selected.result.value, 50_000_000_000);
    check!(eq; selected.result.normalized, Some(50.0));
    check!(eq; selected.result.mark, seconds("50.00")?);
    check!(eq; selected.result.original_mark, Some(seconds("52.00")?));
    check!(eq; selected.result.adjustment_kind, Some(AdjustmentKind::TrackSizeConversion));
    check!(eq; selected.result.adjustment_basis, Some(AdjustmentBasis::Known));
    check!(eq; selected.result.adjustment_note, Some("Converted from 52.00 for Track Size.".into()));
    check!(eq; selected.result.adjustment_source, Some(SourceRef::new(
        "wiaa_results", Some("https://example.test/coverage".into()),
    )));
    check!(eq; selected.source.performance_id, performance.id);
    check!(selected.conflicts.is_empty());
    Ok(())
}

fn same_dataset(dataset: &ExportDataset, expected: &[u8], witness: &Witness) -> TestResult {
    check_adjustment(dataset)?;
    check!(eq; serialized_bytes(dataset)?, expected);
    check!(eq; serialized_digest(dataset)?, dataset.lineage.input_digest);
    witness.dataset_unchanged(dataset)
}

fn verify_semantics(directory: &Path, captured: &Captured) -> TestResult {
    verify_sidecars::verify(directory, &captured.dataset, &captured.options, 3)?;
    Ok(())
}

fn metadata_refused(directory: &Path, captured: &Captured, name: &str) -> TestResult {
    check!(
        matches!(
            verify_sidecars::verify(directory, &captured.dataset, &captured.options, 3),
            Err(ReportError::Invariant { .. })
        ),
        "mutation {name}"
    );
    Ok(())
}

fn rewrite_csv(path: &Path, original: &[u8], column: &str, forged: &str) -> TestResult {
    let mut reader = csv::Reader::from_reader(original);
    let headers = reader.headers()?.clone();
    let index = headers
        .iter()
        .position(|header| header == column)
        .ok_or("CSV column missing")?;
    let records = reader.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; records.len(), 1);
    let mut row = records
        .first()
        .ok_or("CSV best missing")?
        .iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    *row.get_mut(index).ok_or("CSV cell missing")? = forged.into();
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(Vec::new());
    writer.write_record(&headers)?;
    writer.write_record(&row)?;
    std::fs::write(path, writer.into_inner()?)?;
    Ok(())
}

fn check_sidecar_metadata(directory: &Path, captured: &Captured) -> TestResult {
    let mut reader = csv::Reader::from_path(directory.join(CSV))?;
    let headers = reader.headers()?.clone();
    let records = reader.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; records.len(), 1);
    let record = records.first().ok_or("CSV adjusted best missing")?;
    for (name, expected) in [
        ("best_mark", "50.00"),
        ("original_mark", "52.00"),
        ("best_value", "50000000000"),
        ("adjustment_kind", "track_size_conversion"),
        ("adjustment_basis", "known"),
        ("adjustment_note", "Converted from 52.00 for Track Size."),
        (
            "adjustment_source",
            "{\"id\":\"wiaa_results\",\"url\":\"https://example.test/coverage\"}",
        ),
    ] {
        let index = headers
            .iter()
            .position(|header| header == name)
            .ok_or("metadata header missing")?;
        check!(eq; record.get(index), Some(expected));
    }
    let text = std::fs::read_to_string(directory.join(JSONL))?;
    check!(eq; text.lines().count(), 1);
    let found: Value = serde_json::from_str(text.trim())?;
    let rows = selected_bests(&captured.dataset)?;
    check!(eq; found, serde_json::to_value(rows.first().ok_or("adjusted selection missing")?)?);
    verify_semantics(directory, captured)
}

fn sidecar_forgeries_are_refused(captured: &Captured) -> TestResult {
    let scratch = tempfile::tempdir()?;
    let generation = captured
        .workbook
        .parent()
        .ok_or("publication directory missing")?;
    for entry in std::fs::read_dir(generation)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == std::ffi::OsStr::new("workbook.xlsx") || !entry.file_type()?.is_file() {
            continue;
        }
        std::fs::copy(entry.path(), scratch.path().join(&name))?;
    }
    check_sidecar_metadata(scratch.path(), captured)?;
    let csv_path = scratch.path().join(CSV);
    let original_csv = std::fs::read(&csv_path)?;
    for (column, forged) in [
        ("original_mark", "53.00"),
        ("adjustment_kind", ""),
        ("adjustment_basis", "unknown"),
        ("adjustment_note", "forged conversion note"),
        (
            "adjustment_source",
            "{\"id\":\"foreign\",\"url\":\"https://example.test/foreign\"}",
        ),
    ] {
        rewrite_csv(&csv_path, &original_csv, column, forged)?;
        metadata_refused(scratch.path(), captured, CSV)?;
    }
    std::fs::write(&csv_path, original_csv)?;
    let jsonl_path = scratch.path().join(JSONL);
    let original_jsonl = std::fs::read_to_string(&jsonl_path)?;
    for (field, forged) in [
        ("original_mark", serde_json::to_value(seconds("53.00")?)?),
        ("adjustment_kind", Value::Null),
        ("adjustment_basis", serde_json::json!("unknown")),
        (
            "adjustment_note",
            serde_json::json!("forged conversion note"),
        ),
        (
            "adjustment_source",
            serde_json::to_value(SourceRef::new(
                "foreign",
                Some("https://example.test/foreign".into()),
            ))?,
        ),
    ] {
        let mut found: Value = serde_json::from_str(original_jsonl.trim())?;
        *found
            .get_mut(field)
            .ok_or("JSONL adjustment metadata missing")? = forged;
        std::fs::write(&jsonl_path, format!("{}\n", serde_json::to_string(&found)?))?;
        metadata_refused(scratch.path(), captured, JSONL)?;
    }
    std::fs::write(jsonl_path, original_jsonl)?;
    check_sidecar_metadata(scratch.path(), captured)
}

fn mutate_adjustment(store: &Store) -> TestResult {
    let mut performance = only_performance(store)?;
    performance.adjustment = Some(adjustment("53.00")?);
    store.append(Table::Performances, &performance)?;
    check!(!only_performance(store)?.retained_conflicts.is_empty());
    let live = ExportDataset::load(store)?;
    check!(selected_bests(&live)?.is_empty());
    Ok(())
}

#[test]
fn adjusted_bests_and_nonempty_coverage_survive_json_chunks_publication_and_live_mutation(
) -> TestResult {
    let captured = adjusted_capture()?;
    check_adjustment(&captured.dataset)?;
    let expected = serialized_bytes(&captured.dataset)?;
    let witness = Witness::capture(&captured)?;
    let live = ExportDataset::load(&captured.store)?;
    check_adjustment(&live)?;
    check!(eq; serialized_bytes(&live)?, expected);
    check!(eq; live.lineage.input_digest, captured.dataset.lineage.input_digest);
    drop(live);
    let manifest = captured.frozen.manifest();
    check!(eq; manifest.streams.len(), 15);
    check!(eq; manifest.source_fingerprints.len(), 16);
    check!(eq; manifest.lineage, captured.dataset.lineage);
    check!(eq; manifest.legacy_logical_digest.as_deref(), Some(captured.dataset.lineage.input_digest.as_str()));
    same_dataset(
        &to_export_dataset(&captured.frozen, &captured.store)?,
        &expected,
        &witness,
    )?;
    same_dataset(
        &ExportDataset::reopen_frozen(&captured.archive)?,
        &expected,
        &witness,
    )?;
    sidecar_forgeries_are_refused(&captured)?;
    fixture::mutate(&captured.store, Mutation::Both)?;
    mutate_adjustment(&captured.store)?;
    readback::assert_live_changed(&captured.store, Mutation::Both)?;
    same_dataset(&captured.dataset, &expected, &witness)?;
    check!(
        matches!(captured.dataset.ensure_snapshot(&captured.store.snapshot()),
        Err(ReportError::Invariant { ref detail }) if detail.contains("stale or foreign"))
    );
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
    let reopened = Store::open(root)?;
    readback::assert_live_changed(&reopened, Mutation::Both)?;
    check!(selected_bests(&ExportDataset::load(&reopened)?)?.is_empty());
    let archived = ExportDataset::reopen_frozen(&archive)?;
    same_dataset(&archived, &expected, &witness)?;
    same_dataset(&to_export_dataset(&frozen, &reopened)?, &expected, &witness)?;
    let reused = freeze_v2(&reopened, JOB)?;
    assert_same_root(&frozen, &reused)?;
    same_dataset(&to_export_dataset(&reused, &reopened)?, &expected, &witness)?;
    witness.publication_unchanged(&reopened, &workbook, &archive)?;
    crate::workbook::verify::verify_frozen(&workbook, &archived, &options)?;
    let opened = open_publication(
        &workbook,
        &archived.lineage,
        Scope::AllSources,
        GradYear::CO2027,
    )?;
    same_dataset(&opened.dataset, &expected, &witness)?;
    check!(eq; opened.generation_digest, witness.generation);
    check!(directory.path().is_dir());
    Ok(())
}
