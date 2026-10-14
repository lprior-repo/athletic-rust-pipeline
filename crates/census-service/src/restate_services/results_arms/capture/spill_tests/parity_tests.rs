use super::*;
use crate::restate_services::results_arms::capture::Cache;
use census_store::{Store, Table, MAX_OBSERVATION_RECORD_BYTES};

#[test]
fn excluded_rows_never_retain_selected_heap_and_disk_cache_retains_only_descriptors() -> TestResult
{
    let root = tempfile::tempdir()?;
    let rows = (0..512)
        .map(|index| {
            let mut row = meet(
                UsJurisdiction::Wisconsin,
                &format!("{index:06}"),
                Some("2023-04-01"),
            );
            row.name = "x".repeat(16 * 1024);
            row
        })
        .collect();
    let (dataset, _) = build(root.path(), rows)?;
    check!(dataset.disk_bytes > 8 * 1024 * 1024);
    check!(dataset.retained_bytes < 1024 * 1024);
    let admission = Arc::new(Admission::default());
    let selected = selected(dataset, Arc::clone(&admission))?;
    check!(selected.meets.is_empty());
    check!(selected.pending.is_empty());
    check!(eq; admission.selected_usage().0, 0);
    check!(admission.selected_usage().1 < 1024);
    drop(selected);
    check!(eq; admission.selected_usage(), (0, 0));
    Ok(())
}

#[test]
fn duplicate_pending_urls_are_charged_before_sort_and_dedup() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut first = meet(UsJurisdiction::Wisconsin, "1", None);
    first.results_url = "historical/raw/locator".to_string();
    let mut second = meet(UsJurisdiction::Wisconsin, "2", Some("2024-02-30"));
    second.results_url = first.results_url.clone();
    let (dataset, _) = build(root.path(), vec![first, second])?;
    let admission = Arc::new(Admission::default());
    let selected = selected(dataset, Arc::clone(&admission))?;
    check!(eq; selected.pending, vec!["historical/raw/locator".to_string()]);
    check!(eq; admission.selected_usage().0, 2);
    drop(selected);
    check!(eq; admission.selected_usage(), (0, 0));
    Ok(())
}

#[test]
fn duplicate_pending_urls_cannot_bypass_the_exact_selection_row_ceiling() -> TestResult {
    let root = tempfile::tempdir()?;
    let spill = Arc::new(SpillAdmission::default());
    let mut build = Build::new(root.path(), spill)?;
    (0..=budget::SELECTION_ROWS).try_for_each(|index| {
        let mut row = meet(UsJurisdiction::Wisconsin, &format!("{index:08}"), None);
        row.results_url = "same locator".to_string();
        build.push(row, &uncancelled())
    })?;
    let dataset = Arc::new(build.finish(&uncancelled())?);
    let admission = Arc::new(Admission::default());
    let error = selected(dataset, Arc::clone(&admission))
        .err()
        .ok_or("extra pending row accepted")?;
    check!(matches!(
        error,
        StoreError::BudgetExceeded {
            what: "results selected rows",
            needed: 65_537,
            available: 65_536,
        }
    ));
    check!(eq; admission.selected_usage(), (0, 0));
    Ok(())
}

#[test]
fn malformed_some_date_and_first_jurisdiction_preserve_existing_merge_ownership() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let mut first = meet(UsJurisdiction::Wisconsin, "1", Some("2024-02-30"));
    first.name.clear();
    first.results_url.clear();
    let mut later = meet(UsJurisdiction::Illinois, "1", Some("2024-04-01"));
    later.observed_on = "2026-10-01".to_string();
    store.append(Table::SourceMeets, &first)?;
    store.append(Table::SourceMeets, &later)?;
    let mut expected = Vec::new();
    store.for_each_merged(Table::SourceMeets, |row: SourceMeetRef| {
        expected.push(row);
        Ok(())
    })?;
    let mut cache = Cache::default();
    let (selected, measured) =
        cache.select(&store, scope(UsJurisdiction::Wisconsin)?, &uncancelled())?;
    let merged = expected.first().ok_or("merged fixture")?;
    check!(eq; merged.jurisdiction, UsJurisdiction::Wisconsin);
    check!(eq; merged.date.as_deref(), Some("2024-02-30"));
    check!(eq; merged.observed_on.as_str(), "2026-10-01");
    check!(selected.meets.is_empty());
    check!(eq; selected.pending, vec![merged.results_url.clone()]);
    check!(eq; measured.capture.decoded_rows, 2);
    check!(eq; measured.spill_decoded_rows, 1);
    let (foreign, measured) =
        cache.select(&store, scope(UsJurisdiction::Illinois)?, &uncancelled())?;
    check!(foreign.meets.is_empty() && foreign.pending.is_empty());
    check!(eq; measured.capture.decoded_rows, 0);
    check!(eq; measured.spill_decoded_rows, 0);
    Ok(())
}

#[test]
fn independently_valid_physical_rows_can_merge_past_one_mib() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let mut first = meet(UsJurisdiction::Wisconsin, "1", Some("2024-04-01"));
    first.name = "n".repeat(600 * 1024);
    first.venue.clear();
    let mut later = meet(UsJurisdiction::Wisconsin, "1", Some("2024-04-01"));
    later.name.clear();
    later.venue = "v".repeat(600 * 1024);
    check!(serde_json::to_vec(&first)?.len() < MAX_OBSERVATION_RECORD_BYTES);
    check!(serde_json::to_vec(&later)?.len() < MAX_OBSERVATION_RECORD_BYTES);
    store.append(Table::SourceMeets, &first)?;
    store.append(Table::SourceMeets, &later)?;
    let mut expected = Vec::new();
    store.for_each_merged(Table::SourceMeets, |row: SourceMeetRef| {
        expected.push(row);
        Ok(())
    })?;
    check!(
        serde_json::to_vec(expected.first().ok_or("merged fixture")?)?.len()
            > MAX_OBSERVATION_RECORD_BYTES
    );
    let (selected, measured) =
        Cache::default().select(&store, scope(UsJurisdiction::Wisconsin)?, &uncancelled())?;
    check!(eq; selected.meets, expected);
    check!(eq; measured.capture.decoded_rows, 2);
    check!(eq; measured.spill_decoded_rows, 1);
    check!(measured.spill_peak_scratch_bytes <= budget::SPILL_SCRATCH_BYTES);
    Ok(())
}

#[test]
fn selected_vectors_and_canonical_plan_bytes_match_existing_pure_selection() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let rows = vec![
        meet(UsJurisdiction::Wisconsin, "0001", Some("2024-04-01")),
        meet(UsJurisdiction::Wisconsin, "2", None),
        meet(UsJurisdiction::Wisconsin, "3", Some("2024-02-30")),
        meet(UsJurisdiction::Wisconsin, "4", Some("2023-04-01")),
        meet(UsJurisdiction::Illinois, "5", Some("2024-04-01")),
    ];
    store.append_many(Table::SourceMeets, &rows)?;
    let stage = scope(UsJurisdiction::Wisconsin)?;
    let mut expected_meets = Vec::new();
    let mut expected_pending = Vec::new();
    store.for_each_merged(Table::SourceMeets, |row: SourceMeetRef| {
        if row.jurisdiction == stage.jurisdiction {
            match selection::assess(&row, stage) {
                selection::Admission::Admitted => expected_meets.push(row),
                selection::Admission::Unknown => expected_pending.push(row.results_url),
                selection::Admission::Excluded => (),
            }
        }
        Ok(())
    })?;
    expected_pending.sort();
    expected_pending.dedup();
    let (selected, _) = Cache::default().select(&store, stage, &uncancelled())?;
    check!(eq; selected.meets, expected_meets);
    check!(eq; selected.pending, expected_pending);
    check!(eq; plan_bytes(selected.meets.clone(), selected.pending.clone())?, plan_bytes(expected_meets, expected_pending)?);
    Ok(())
}

fn plan_bytes(meets: Vec<SourceMeetRef>, pending: Vec<String>) -> TestResult<bytes::Bytes> {
    use crate::restate_services::results_arms::plan_v2::{
        Discovery, DiscoveryFailure, DiscoveryFailureCategory, MeetRoute, PlanBuilder,
        ResultsPlanBinding, ResultsPlanV2, SelectionUsage,
    };
    use census_domain::model::{
        AcquisitionDigest, CaptureCacheIntent, CensusRun, ResultsCaptureScopeV1, SchoolYear,
        SelectedHistoryV1, SourceAdmissionV1, StoreIdentity,
    };
    let stage = scope(UsJurisdiction::Wisconsin)?;
    let capture = ResultsCaptureScopeV1::new(
        "wi-2026-r1",
        stage.jurisdiction,
        CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("fixture run")?,
        AcquisitionDigest::parse(&"b".repeat(64))?,
        SelectedHistoryV1::new(stage.window, stage.year)?,
        SourceAdmissionV1::milesplit_results(&"c".repeat(64))?,
        CaptureCacheIntent::ReuseEligible,
    )?;
    let binding = ResultsPlanBinding::new(
        StoreIdentity::parse(&"a".repeat(64))?,
        capture,
        "2026-10-08".to_string(),
    )?;
    let count = meets.len();
    let mut builder = PlanBuilder::new(binding, vec!["milesplit".to_string()], meets, pending)?;
    (0..count).try_for_each(|_| {
        builder.record_meet(
            Discovery::Failed(DiscoveryFailure {
                category: DiscoveryFailureCategory::TargetMismatch,
                detail: "invalid structured meet ID".to_string(),
            }),
            MeetRoute::NotOwned,
        )
    })?;
    let usage = census_store::AcquisitionReadBudget::for_selection().selection_usage();
    let plan = builder.finish(SelectionUsage::observed(&usage)?)?;
    Ok(<ResultsPlanV2 as restate_sdk::serde::Serialize>::serialize(
        &plan,
    )?)
}

#[test]
fn spill_replay_preserves_original_capacity_charges_after_reallocation() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut row = meet(UsJurisdiction::Wisconsin, "1", Some("2024-04-01"));
    row.name.try_reserve_exact(8192)?;
    let original_heap = budget::heap_bytes(&row)?;
    let expected = budget::slots::<selection::Selected>(1)?
        .checked_add(budget::slots::<SourceMeetRef>(budget::VECTOR_CHUNK)?)
        .ok_or("slot overflow")?
        .checked_add(original_heap)
        .ok_or("heap overflow")?;
    let (dataset, _) = build(root.path(), vec![row])?;
    let admission = Arc::new(Admission::default());
    let selected = selected(dataset, Arc::clone(&admission))?;
    check!(budget::heap_bytes(selected.meets.first().ok_or("selected meet")?)? < original_heap);
    check!(eq; admission.selected_usage(), (1, expected));
    drop(selected);
    check!(eq; admission.selected_usage(), (0, 0));
    Ok(())
}

#[test]
fn duplicate_heavy_capture_cancels_at_merged_publication_without_claiming_physical_checkpoints(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    (0..64).try_for_each(|_| {
        store
            .append(
                Table::SourceMeets,
                &meet(UsJurisdiction::Wisconsin, "1", None),
            )
            .map(|_| ())
    })?;
    let cancelled = AtomicBool::new(true);
    let mut physical_at_checkpoint = 0_u64;
    let result = store.snapshot().for_each_merged_bounded(
        Table::SourceMeets,
        census_store::read::ReadBudget {
            rows: budget::CAPTURE_ROWS,
            bytes: budget::CAPTURE_BYTES,
            record_bytes: MAX_OBSERVATION_RECORD_BYTES,
        },
        |_row: SourceMeetRef, encoded| {
            physical_at_checkpoint = encoded;
            crate::restate_services::results_arms::capture::check_cancel(&cancelled)
        },
    );
    check!(result.is_err());
    let physical = store.snapshot().for_each_observation_bounded(
        Table::SourceMeets,
        census_store::read::ReadBudget {
            rows: budget::CAPTURE_ROWS,
            bytes: budget::CAPTURE_BYTES,
            record_bytes: MAX_OBSERVATION_RECORD_BYTES,
        },
        |_row: SourceMeetRef, _encoded| Ok(()),
    )?;
    check!(eq; physical.decoded_rows, 64);
    check!(eq; physical_at_checkpoint, physical.encoded_bytes);
    Ok(())
}

#[test]
fn historical_embedded_nul_ids_keep_exact_store_publication_order() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    ["a", "a\0", "a", "a\0"].into_iter().try_for_each(|id| {
        let mut row = meet(UsJurisdiction::Wisconsin, id, Some("2024-04-01"));
        row.name = "名字 \"quoted\"\nline\\slash".to_string();
        row.venue = "🏟\tvenue\r".to_string();
        row.results_url.push_str("?raw=\"\0\\");
        store.append(Table::SourceMeets, &row)
    })?;
    let mut expected = Vec::new();
    store.for_each_merged(Table::SourceMeets, |row: SourceMeetRef| {
        expected.push(row);
        Ok(())
    })?;
    check!(expected.iter().any(|row| row.id.contains('\0')));
    let (selected, measured) =
        Cache::default().select(&store, scope(UsJurisdiction::Wisconsin)?, &uncancelled())?;
    check!(eq; selected.meets, expected);
    check!(eq; measured.capture.decoded_rows, 4);
    check!(eq; measured.spill_decoded_rows, u64::try_from(expected.len())?);
    Ok(())
}
