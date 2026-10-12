use std::sync::Arc;

use census_domain::{model::SourceMeetRef, UsJurisdiction};
use census_store::{Store, Table};

use super::super::{history_stage::HistoricalStageScope, wire::HistoryWindow};
use super::{capture::Cache, selection};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn scope(jurisdiction: UsJurisdiction) -> TestResult<HistoricalStageScope> {
    Ok(HistoricalStageScope {
        jurisdiction,
        year: 2024,
        refresh: false,
        window: HistoryWindow::new(2023, 2026, "2026-10-07")?,
        observed_on: chrono::NaiveDate::parse_from_str("2026-10-08", "%Y-%m-%d")?,
    })
}

pub(super) fn meet(jurisdiction: UsJurisdiction, id: &str, date: Option<&str>) -> SourceMeetRef {
    SourceMeetRef {
        id: SourceMeetRef::row_id("milesplit", id),
        source: "milesplit".to_string(),
        source_meet_id: id.to_string(),
        jurisdiction,
        season: "outdoor".to_string(),
        year: 2024,
        name: "Owned meet".to_string(),
        date: date.map(str::to_string),
        venue: "Published venue".to_string(),
        results_url: format!(
            "https://{}.milesplit.com/meets/{id}/results",
            jurisdiction.code().to_ascii_lowercase()
        ),
        observed_on: "2026-10-08".to_string(),
    }
}

#[test]
fn results_stage_reads_only_its_jurisdiction_meets() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Arc::new(Store::open(dir.path())?);
    for (jurisdiction, id) in [
        (UsJurisdiction::Wisconsin, "1"),
        (UsJurisdiction::Illinois, "2"),
    ] {
        store.append(
            Table::SourceMeets,
            &meet(jurisdiction, id, Some("2024-04-01")),
        )?;
        store.append(
            Table::SourceMeets,
            &meet(jurisdiction, &format!("{id}0"), None),
        )?;
    }
    let mut cache = Cache::default();
    let (wi, first) = cache.select(&store, scope(UsJurisdiction::Wisconsin)?)?;
    let (il, second) = cache.select(&store, scope(UsJurisdiction::Illinois)?)?;
    println!(
        "capture first={:?} second={:?}; partition visits wi={} il={}",
        first.capture, second.capture, first.partition_visits, second.partition_visits
    );
    check!(eq; first.capture.decoded_rows, 4);
    check!(eq; second.capture.decoded_rows, 0);
    check!(eq; second.capture.key_visits, 0);
    check!(eq; first.partition_visits, 2);
    check!(eq; second.partition_visits, 2);
    check!(eq; wi.meets, vec![meet(UsJurisdiction::Wisconsin, "1", Some("2024-04-01"))]);
    check!(eq; il.meets, vec![meet(UsJurisdiction::Illinois, "2", Some("2024-04-01"))]);
    check!(eq; wi.pending, vec![meet(UsJurisdiction::Wisconsin, "10", None).results_url]);
    check!(eq; il.pending, vec![meet(UsJurisdiction::Illinois, "20", None).results_url]);
    Ok(())
}

#[test]
fn captured_results_match_existing_merged_selection_and_unfinished_accounting() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Arc::new(Store::open(dir.path())?);
    let first = meet(UsJurisdiction::Wisconsin, "1", None);
    store.append(Table::SourceMeets, &first)?;
    store.append(
        Table::SourceMeets,
        &meet(UsJurisdiction::Wisconsin, "1", Some("2024-04-01")),
    )?;
    store.append(
        Table::SourceMeets,
        &meet(UsJurisdiction::Wisconsin, "2", None),
    )?;
    store.append(
        Table::SourceMeets,
        &meet(UsJurisdiction::Wisconsin, "3", Some("2023-04-01")),
    )?;
    store.append(
        Table::SourceMeets,
        &meet(UsJurisdiction::Illinois, "4", Some("2024-04-01")),
    )?;
    let scope = scope(UsJurisdiction::Wisconsin)?;
    let mut admitted = Vec::new();
    let mut pending = Vec::new();
    store.for_each_merged(Table::SourceMeets, |row: SourceMeetRef| {
        if row.jurisdiction == scope.jurisdiction {
            match selection::assess(&row, scope) {
                selection::Admission::Admitted => admitted.push(row),
                selection::Admission::Unknown => pending.push(row.results_url),
                selection::Admission::Excluded => (),
            }
        }
        Ok(())
    })?;
    pending.sort();
    pending.dedup();
    let (selected, _) = Cache::default().select(&store, scope)?;
    check!(eq; selected.meets, admitted);
    check!(eq; selected.pending, pending);
    check!(eq; selected.meets.len(), 1);
    check!(eq; selected.pending.len(), 1);
    Ok(())
}

#[test]
fn advancing_generation_refuses_a_second_live_capture_then_rebuilds_after_release() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Arc::new(Store::open(dir.path())?);
    store.append(
        Table::SourceMeets,
        &meet(UsJurisdiction::Wisconsin, "1", Some("2024-04-01")),
    )?;
    let mut cache = Cache::default();
    let (old, _) = cache.select(&store, scope(UsJurisdiction::Wisconsin)?)?;
    store.append(
        Table::SourceMeets,
        &meet(UsJurisdiction::Wisconsin, "2", None),
    )?;
    check!(matches!(
        cache.select(&store, scope(UsJurisdiction::Wisconsin)?),
        Err(census_store::StoreError::BudgetExceeded {
            what: "results live captures",
            needed: 2,
            available: 1
        })
    ));
    drop(old);
    let (current, measured) = cache.select(&store, scope(UsJurisdiction::Wisconsin)?)?;
    check!(eq; measured.capture.decoded_rows, 2);
    check!(eq; current.meets, vec![meet(UsJurisdiction::Wisconsin, "1", Some("2024-04-01"))]);
    check!(eq; current.pending, vec![meet(UsJurisdiction::Wisconsin, "2", None).results_url]);
    Ok(())
}

#[test]
fn captured_partitions_never_cross_store_owners_with_equal_generations() -> TestResult {
    let left_dir = tempfile::tempdir()?;
    let right_dir = tempfile::tempdir()?;
    let left = Arc::new(Store::open(left_dir.path())?);
    let right = Arc::new(Store::open(right_dir.path())?);
    left.append(
        Table::SourceMeets,
        &meet(UsJurisdiction::Wisconsin, "1", None),
    )?;
    right.append(
        Table::SourceMeets,
        &meet(UsJurisdiction::Wisconsin, "2", None),
    )?;
    check!(eq; left.snapshot().evidence_generation(), right.snapshot().evidence_generation());
    let mut cache = Cache::default();
    let (selected, _) = cache.select(&left, scope(UsJurisdiction::Wisconsin)?)?;
    drop(selected);
    let (selected, measured) = cache.select(&right, scope(UsJurisdiction::Wisconsin)?)?;
    check!(eq; measured.capture.decoded_rows, 1);
    check!(eq; selected.pending, vec![meet(UsJurisdiction::Wisconsin, "2", None).results_url]);
    Ok(())
}

#[test]
fn national_fanout_decodes_one_capture_and_visits_only_49_owned_partitions() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Arc::new(Store::open(dir.path())?);
    for jurisdiction in UsJurisdiction::CENSUS_SCOPE {
        let mut rows = Vec::new();
        for index in 0..1024 {
            let date = match index % 4 {
                0 => Some("2024-04-01"),
                1 => None,
                2 => Some("2024-02-30"),
                _ => Some("2023-04-01"),
            };
            rows.push(meet(
                jurisdiction,
                &format!("{}-{index}", jurisdiction.code()),
                date,
            ));
        }
        store.append_many(Table::SourceMeets, &rows)?;
    }
    let mut cache = Cache::default();
    let mut selected = Vec::new();
    let mut decoded = 0_u64;
    let mut keys = 0_u64;
    let mut visits = 0_u64;
    let mut encoded = 0_u64;
    let mut retained = 0_u64;
    let mut refusals = 0_u64;
    let started = std::time::Instant::now();
    for jurisdiction in UsJurisdiction::CENSUS_SCOPE {
        let (partition, measured) = match cache.select(&store, scope(jurisdiction)?) {
            Ok(selected) => selected,
            Err(census_store::StoreError::BudgetExceeded {
                what: "results live captures",
                ..
            }) => {
                refusals = refusals.checked_add(1).ok_or("refusal overflow")?;
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        decoded = decoded
            .checked_add(measured.capture.decoded_rows)
            .ok_or("decode overflow")?;
        keys = keys
            .checked_add(measured.capture.key_visits)
            .ok_or("key overflow")?;
        visits = visits
            .checked_add(measured.partition_visits)
            .ok_or("visit overflow")?;
        encoded = encoded
            .checked_add(measured.capture.encoded_bytes)
            .ok_or("byte overflow")?;
        retained = measured.retained_bytes;
        check!(eq; measured.partition_visits, 1024);
        check!(eq; partition.meets.len(), 256);
        check!(eq; partition.pending.len(), 512);
        check!(partition.meets.iter().all(
            |row| row.jurisdiction == jurisdiction && row.date.as_deref() == Some("2024-04-01")
        ));
        selected.push(partition);
    }
    println!("national jurisdictions={} physical=50176 capture decoded={decoded} keys={keys} encoded_bytes={encoded} retained_bytes={retained} selection_visits={visits} admitted=12544 pending=25088 live_capture_refusals={refusals} elapsed={:?}", selected.len(), started.elapsed());
    check!(eq; decoded, 50_176);
    check!(eq; keys, 50_176);
    check!(eq; visits, 50_176);
    check!(eq; selected.len(), 49);
    check!(eq; refusals, 0);
    Ok(())
}
