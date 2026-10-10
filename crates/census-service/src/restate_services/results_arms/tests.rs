use super::selection::{academic_year, assess, Admission};
use super::*;
use crate::restate_services::wire::HistoryWindow;
use census_domain::model::SourceMeetRef;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn scope(year: u16) -> TestResult<HistoricalStageScope> {
    Ok(HistoricalStageScope {
        jurisdiction: UsJurisdiction::Wisconsin,
        year,
        refresh: false,
        window: HistoryWindow::new(2023, 2026, "2026-10-07")?,
        observed_on: chrono::NaiveDate::parse_from_str("2026-10-08", "%Y-%m-%d")?,
    })
}

fn row(date: Option<&str>, season: &str, year: u16) -> SourceMeetRef {
    SourceMeetRef {
        id: SourceMeetRef::row_id("milesplit", "123"),
        source: "milesplit".to_string(),
        source_meet_id: "123".to_string(),
        jurisdiction: UsJurisdiction::Wisconsin,
        season: season.to_string(),
        year,
        name: "Transfer Invitational".to_string(),
        date: date.map(str::to_string),
        venue: "Former school".to_string(),
        results_url: "https://wi.milesplit.com/meets/123/results".to_string(),
        observed_on: "2026-10-08".to_string(),
    }
}

#[test]
fn historical_outdoor_indoor_and_xc_use_source_dates_not_current_sweep_season() -> TestResult {
    let examples = [
        ("2023-04-12", "outdoor", 2023, 2022),
        ("2024-02-08", "indoor", 2024, 2023),
        ("2025-09-04", "xc", 2025, 2025),
        ("2026-07-31", "outdoor", 2026, 2025),
        ("2026-08-01", "xc", 2026, 2026),
    ];
    for (date, season, calendar, academic) in examples {
        let meet = row(Some(date), season, 2026);
        check!(eq; assess(&meet, scope(calendar)?), Admission::Admitted);
        check!(eq; academic_year(&meet).map_err(crate::restate_services::tests::sdk_error)?.get(), academic);
    }
    Ok(())
}

#[test]
fn full_asof_date_excludes_future_results_but_admits_boundary_day() -> TestResult {
    check!(eq; assess(&row(Some("2026-10-07"), "xc", 2026), scope(2026)?), Admission::Admitted);
    check!(eq; assess(&row(Some("2026-10-08"), "xc", 2026), scope(2026)?), Admission::Excluded);
    check!(eq; assess(&row(Some("2022-10-07"), "xc", 2022), scope(2023)?), Admission::Excluded);
    Ok(())
}

#[test]
fn unknown_or_malformed_dates_remain_owed_and_cannot_supply_a_grade_year() -> TestResult {
    for date in [None, Some("2026"), Some("2026-02-30")] {
        let meet = row(date, "indoor", 2024);
        check!(eq; assess(&meet, scope(2024)?), Admission::Unknown);
        check!(academic_year(&meet).is_err());
    }
    Ok(())
}

#[test]
fn source_counts_and_empty_reports_never_manufacture_completion() -> TestResult {
    let mut report = AdapterReport::new("milesplit", "performances");
    report.rows = 37;
    let source =
        source_rows("milesplit", 1, report).map_err(crate::restate_services::tests::sdk_error)?;
    check!(eq; source.disposition, CollectionDisposition::Unknown);
    let outcome = ResultsStageOutcome {
        per_source: vec![source],
        pending: Vec::new(),
        required_sources: vec!["milesplit".to_string()],
    };
    check!(!outcome.is_terminal());
    check!(!ResultsStageOutcome::default().is_terminal());
    Ok(())
}

#[test]
fn source_failures_rejections_and_exact_unfinished_locators_block_terminal_results() -> TestResult {
    let mut report = AdapterReport::new("milesplit", "performances");
    report.disposition = CollectionDisposition::Complete;
    let clean =
        source_rows("milesplit", 1, report).map_err(crate::restate_services::tests::sdk_error)?;
    let complete = ResultsStageOutcome {
        per_source: vec![clean.clone()],
        pending: Vec::new(),
        required_sources: vec!["milesplit".to_string()],
    };
    check!(complete.is_terminal());
    let mut variants = Vec::new();
    let mut failed = clean.clone();
    failed.errors = 1;
    variants.push(failed);
    let mut withheld = clean.clone();
    withheld.withheld = Some(1);
    variants.push(withheld);
    let mut unknown = clean.clone();
    unknown.withheld = None;
    variants.push(unknown);
    let mut unresolved = clean.clone();
    unresolved.unresolved = Some(UnresolvedCounters { rows: 1, labels: 0 });
    variants.push(unresolved);
    let mut unfinished = clean.clone();
    unfinished.unfinished = vec!["https://wi.milesplit.com/meets/123/results/7/raw".to_string()];
    variants.push(unfinished);
    let mut unknown_rows = clean.clone();
    unknown_rows.rows = None;
    variants.push(unknown_rows);
    for source in variants {
        let outcome = ResultsStageOutcome {
            per_source: vec![source],
            ..complete.clone()
        };
        check!(!outcome.is_terminal());
    }
    let duplicate = ResultsStageOutcome {
        per_source: vec![clean.clone(), clean],
        ..complete
    };
    check!(!duplicate.is_terminal());
    Ok(())
}

#[test]
fn an_unselected_milesplit_source_performs_no_acquisition() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Arc::new(Store::open(dir.path().join("store"))?);
            let fetcher = Arc::new(Fetcher::new(
                dir.path().join("http"),
                None,
                std::time::Duration::from_millis(1),
                std::collections::HashMap::new(),
                Vec::new(),
            )?);
            let Json(outcome) = results_stage(
                Arc::clone(&store),
                fetcher,
                scope(2024)?,
                SourceSelection::parse(&["coach_directories".to_string()])
                    .map_err(crate::restate_services::tests::sdk_error)?,
            )
            .await
            .map_err(crate::restate_services::tests::sdk_error)?;
            check!(eq; outcome.required_sources, Vec::<String>::new());
            check!(eq; outcome.per_source, Vec::<ResultsSourceRows>::new());
            check!(!outcome.is_terminal());
            check!(eq; store.walk_table(census_store::Table::Performances)?.rows, 0);
            Ok(())
        })
}

#[test]
fn malformed_owned_athleticnet_identifiers_are_not_silently_dropped() -> TestResult {
    let mut meet = row(Some("2024-04-01"), "outdoor", 2024);
    meet.source = ATHLETICNET.to_string();
    meet.source_meet_id = "broken".to_string();
    let result = athleticnet_meet_ids(&[meet], UsJurisdiction::Wisconsin);
    check!(result.is_err());
    check!(eq; meet_id_in("https://results.wayzataresults.com/meet/634313"), None);
    check!(eq; meet_id_in("https://www.athletic.net/TrackAndField/meet/634313/results"), Some(634313));
    Ok(())
}

#[test]
fn malformed_owned_meet_locator_is_quarantined_instead_of_silently_filtered() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Arc::new(Store::open(dir.path().join("store"))?);
            let fetcher = Arc::new(
                Fetcher::new(
                    dir.path().join("http"),
                    None,
                    std::time::Duration::ZERO,
                    std::collections::HashMap::new(),
                    Vec::new(),
                )?
                .with_offline(true),
            );
            let mut meet = row(Some("2024-02-08"), "indoor", 2024);
            meet.results_url = "not a URL".to_string();
            let source = collection::collect(
                "milesplit",
                ResultsArm::MilesplitResults,
                &store,
                &fetcher,
                (std::slice::from_ref(&meet), scope(2024)?),
            )
            .await
            .map_err(crate::restate_services::tests::sdk_error)?;
            check!(eq; source.meets, 1);
            check!(eq; source.errors, 1);
            check!(source
                .unfinished
                .iter()
                .any(|locator| locator == "not a URL"));
            check!(!source_complete(&source));
            check!(eq; store.walk_table(census_store::Table::Performances)?.rows, 0);
            Ok(())
        })
}

#[test]
fn dur05_adapter_errors_are_preserved_in_results_source_rows() -> TestResult {
    let mut report = census_crawl::AdapterReport::new("milesplit_results", "result rows");
    report.rows = 10;
    report.errors = 2;
    let rows = super::source_rows("test_slug", 5, report)
        .map_err(crate::restate_services::tests::sdk_error)?;
    check!(eq; rows.errors, 2);
    let silent = census_crawl::AdapterReport::new("athleticnet", "performances");
    let rows = super::source_rows("test_slug", 5, silent)
        .map_err(crate::restate_services::tests::sdk_error)?;
    check!(eq; rows.errors, 0);
    Ok(())
}

#[test]
fn resolution_counters_are_captured_and_survive_serialization() -> TestResult {
    let mut report = census_crawl::AdapterReport::new("milesplit", "result rows");
    report.rows = 100;
    report.disposition = CollectionDisposition::Complete;
    report.resolution = Some(census_crawl::ResolutionCounters {
        rows: 100,
        resolved: 5,
        unresolved: 1,
        retained: 3,
        quarantined: 0,
    });
    let source =
        source_rows("milesplit", 1, report).map_err(crate::restate_services::tests::sdk_error)?;
    check!(source.resolution.is_some());
    let resolution = source.resolution.unwrap();
    check!(eq; resolution.rows, 100);
    check!(eq; resolution.resolved, 5);
    check!(eq; resolution.unresolved, 1);
    check!(eq; resolution.retained, 3);
    check!(eq; resolution.quarantined, 0);
    let json = serde_json::to_string(&source)?;
    let restored: ResultsSourceRows = serde_json::from_str(&json)?;
    check!(restored.resolution.is_some());
    check!(eq; restored.resolution.unwrap().unresolved, 1);
    Ok(())
}

#[test]
fn legacy_source_rows_without_resolution_key_deserialize() -> TestResult {
    let legacy = r#"{"slug":"milesplit","meets":1,"rows":100,"disposition":"Complete","errors":0,"withheld":0,"notes":[],"unfinished":[],"unresolved":{"rows":0,"labels":0}}"#;
    let source: ResultsSourceRows = serde_json::from_str(legacy)?;
    check!(eq; source.slug, "milesplit");
    check!(source.resolution.is_none());
    Ok(())
}

#[test]
fn unresolved_resolution_counters_block_terminal_status() -> TestResult {
    let mut report = census_crawl::AdapterReport::new("milesplit", "result rows");
    report.rows = 100;
    report.disposition = CollectionDisposition::Complete;
    report.resolution = Some(census_crawl::ResolutionCounters {
        rows: 100,
        resolved: 5,
        unresolved: 1,
        retained: 0,
        quarantined: 0,
    });
    let source =
        source_rows("milesplit", 1, report).map_err(crate::restate_services::tests::sdk_error)?;
    let outcome = ResultsStageOutcome {
        per_source: vec![source],
        pending: Vec::new(),
        required_sources: vec!["milesplit".to_string()],
    };
    check!(!outcome.is_terminal());
    let mut clean_report = census_crawl::AdapterReport::new("milesplit", "result rows");
    clean_report.rows = 100;
    clean_report.disposition = CollectionDisposition::Complete;
    clean_report.resolution = Some(census_crawl::ResolutionCounters::default());
    let clean_source = source_rows("milesplit", 1, clean_report)
        .map_err(crate::restate_services::tests::sdk_error)?;
    let clean_outcome = ResultsStageOutcome {
        per_source: vec![clean_source],
        pending: Vec::new(),
        required_sources: vec!["milesplit".to_string()],
    };
    check!(clean_outcome.is_terminal());
    Ok(())
}

#[test]
fn resolution_counter_merge_overflow_returns_invariant() -> TestResult {
    let left = ResultsSourceRows {
        slug: "test".to_string(),
        meets: 1,
        rows: Some(1),
        disposition: CollectionDisposition::Complete,
        errors: 0,
        withheld: Some(0),
        notes: Vec::new(),
        unfinished: Vec::new(),
        unresolved: Some(UnresolvedCounters { rows: 0, labels: 0 }),
        resolution: Some(census_crawl::ResolutionCounters {
            rows: u64::MAX,
            resolved: 0,
            unresolved: 0,
            retained: 0,
            quarantined: 0,
        }),
    };
    let right = ResultsSourceRows {
        slug: "test".to_string(),
        meets: 1,
        rows: Some(1),
        disposition: CollectionDisposition::Complete,
        errors: 0,
        withheld: Some(0),
        notes: Vec::new(),
        unfinished: Vec::new(),
        unresolved: Some(UnresolvedCounters { rows: 0, labels: 0 }),
        resolution: Some(census_crawl::ResolutionCounters {
            rows: 1,
            resolved: 0,
            unresolved: 0,
            retained: 0,
            quarantined: 0,
        }),
    };
    let result = collection::merge(left, right);
    check!(result.is_err());
    let error = result.err().unwrap();
    check!(format!("{error:?}").contains("resolution row counter overflow"));
    Ok(())
}

#[test]
fn tfrrs_is_an_armed_results_source_for_applicable_states() -> TestResult {
    check!(matches!(arm_for("tfrrs"), Some(ResultsArm::TfrrsResults)));
    Ok(())
}
