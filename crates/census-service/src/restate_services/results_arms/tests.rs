use census_crawl::net::Fetcher;
use census_crawl::AdapterContext;
use census_domain::model::SchoolYear;
use census_domain::model::SourceMeetRef;
use census_domain::UsJurisdiction;
use restate_sdk::prelude::Json;

use super::{arm_for, athleticnet_meet_ids, athleticnet_meets, meet_id_in, ResultsArm};
use crate::restate_services::tests::sdk_error;
use census_crawl::milesplit::is_results_page;
use census_store::Store;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn row(source: &str, id: &str, url: &str, jurisdiction: UsJurisdiction) -> SourceMeetRef {
    SourceMeetRef {
        id: SourceMeetRef::row_id(source, id),
        source: source.to_string(),
        source_meet_id: id.to_string(),
        jurisdiction,
        season: "outdoor".to_string(),
        year: 2026,
        name: "Example Invitational".to_string(),
        date: Some("2026-05-02".to_string()),
        venue: "Example High School".to_string(),
        results_url: url.to_string(),
        observed_on: "2026-09-23".to_string(),
    }
}

#[test]
fn a_foreign_host_is_never_read_as_an_athleticnet_meet() {
    assert_eq!(
        meet_id_in("https://results.wayzataresults.com/meet/634313"),
        None
    );
    assert_eq!(
        meet_id_in("https://www.athletic.net/TrackAndField/meet/634313/results"),
        Some(634313)
    );
    assert_eq!(meet_id_in("https://athletic.net/meet/634313"), Some(634313));
    assert_eq!(
        meet_id_in("https://Athletic.Net/xc/meet/634313/"),
        Some(634313)
    );
    assert_eq!(meet_id_in("https://www.athletic.net/xc/meet/unknown"), None);
    assert_eq!(meet_id_in("https://www.athletic.net/xc/634313"), None);
}

#[test]
fn the_seed_reads_both_row_shapes_and_only_this_jurisdiction() {
    let rows = vec![
        row(
            "athleticnet",
            "634313",
            "https://www.athletic.net/TrackAndField/meet/634313/results",
            UsJurisdiction::Wisconsin,
        ),
        row(
            "wiaa_results",
            "wi-1",
            "https://www.athletic.net/TrackAndField/meet/700001/results",
            UsJurisdiction::Wisconsin,
        ),
        row(
            "mshsl",
            "mn-9",
            "https://www.athletic.net/TrackAndField/meet/700001/results",
            UsJurisdiction::Wisconsin,
        ),
        row(
            "wiaa_results",
            "wi-2",
            "https://www.athletic.net/TrackAndField/meet/800001/results",
            UsJurisdiction::Minnesota,
        ),
        row(
            "milesplit",
            "770621",
            "https://oh.milesplit.com/meets/770621/results",
            UsJurisdiction::Wisconsin,
        ),
        row(
            "athleticnet",
            "not-an-id",
            "https://www.athletic.net/meet/900001",
            UsJurisdiction::Wisconsin,
        ),
    ];
    assert_eq!(
        athleticnet_meet_ids(&rows, UsJurisdiction::Wisconsin),
        vec![634_313, 700_001]
    );
    assert!(athleticnet_meet_ids(&rows, UsJurisdiction::Ohio).is_empty());
}

#[test]
fn a_meet_index_walk_is_not_a_results_arm() {
    assert_eq!(arm_for("wiaa_results"), None);
    assert_eq!(arm_for("wayzata"), None);
    assert_eq!(arm_for("milesplit"), Some(ResultsArm::MilesplitResults));
    assert_eq!(arm_for("athleticnet"), Some(ResultsArm::AthleticnetMeets));
}

#[test]
fn only_a_milesplit_results_page_is_read_by_the_result_set_arm() {
    assert!(is_results_page(
        "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results"
    ));
    assert!(is_results_page("https://WI.MileSplit.COM/meets/1/results"));
    assert!(!is_results_page(
        "https://oh.milesplit.com/meets/770621-x/results/1321880/raw"
    ));
    assert!(!is_results_page(
        "https://www.wiaawi.org/Results/Track/2026/d1boysstateresults.htm"
    ));
    assert!(!is_results_page(
        "https://www.athletic.net/TrackAndField/meet/634313/results"
    ));
    assert!(!is_results_page("https://oh.milesplit.com/teams"));
    assert!(!is_results_page("not a url"));
}

fn scratch() -> TestResult<(tempfile::TempDir, Store, Fetcher)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )?;
    Ok((dir, store, fetcher))
}

fn context<'a>(store: &'a Store, fetcher: &'a Fetcher) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        observed_on: "2026-09-24".to_string(),
        recording: None,
    })
}

#[test]
fn a_selection_that_names_no_meet_is_not_a_pull() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher) = scratch()?;
            let rows = vec![row(
                "milesplit",
                "770621",
                "https://oh.milesplit.com/meets/770621/results",
                UsJurisdiction::Alabama,
            )];
            let (meets, report) = athleticnet_meets(
                &context(&store, &fetcher)?,
                &rows,
                UsJurisdiction::Alabama,
                "2026-09-24",
            )
            .await
            .map_err(sdk_error)?;
            check!(eq; meets, 0);
            check!(eq; report.adapter, "athleticnet");
            check!(eq; report.unit, "performances");
            check!(eq; report.rows, 0);
            check!(eq; report.requests, 0, "nothing to pull means no request");
            Ok(())
        })
}

#[test]
fn an_unarmed_sweepable_slug_fails_closed() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher) = scratch()?;
            let result = super::results_stage(
                std::sync::Arc::new(store),
                std::sync::Arc::new(fetcher),
                UsJurisdiction::Alabama,
                2026,
                false,
                "2026-09-24".to_string(),
                vec!["never_armed_slug".to_string()],
            )
            .await;
            check!(
                result.is_err(),
                "a sweepable slug with no stage arm must fail closed"
            );
            Ok(())
        })
}

#[test]
fn a_slug_armed_by_another_stage_is_not_a_results_pull() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher) = scratch()?;
            let Json(outcome) = super::results_stage(
                std::sync::Arc::new(store),
                std::sync::Arc::new(fetcher),
                UsJurisdiction::Alabama,
                2026,
                false,
                "2026-09-24".to_string(),
                vec!["coach_directories".to_string()],
            )
            .await
            .map_err(sdk_error)?;
            check!(outcome.per_source.is_empty());
            Ok(())
        })
}

#[test]
fn the_outcome_carries_the_reports_unresolved_counters() -> TestResult {
    let mut report = census_crawl::AdapterReport::new("milesplit_results", "result rows");
    report.rows = 7;
    report.unresolved = Some(census_crawl::UnresolvedCounters { rows: 4, labels: 2 });
    let rows = super::source_rows("milesplit", 3, &report).map_err(sdk_error)?;
    check!(eq; rows.slug, "milesplit");
    check!(eq; rows.meets, 3);
    check!(eq; rows.rows, 7);
    check!(eq;
        rows.unresolved,
        Some(census_crawl::UnresolvedCounters { rows: 4, labels: 2 })
    );
    let silent = census_crawl::AdapterReport::new("athleticnet", "performances");
    let rows = super::source_rows("athleticnet", 0, &silent).map_err(sdk_error)?;
    check!(eq; rows.unresolved, None);
    Ok(())
}

#[test]
fn dur05_adapter_errors_are_preserved_in_results_source_rows() -> TestResult {
    let mut report = census_crawl::AdapterReport::new("milesplit_results", "result rows");
    report.rows = 10;
    report.errors = 2;
    let rows = super::source_rows("test_slug", 5, &report).map_err(sdk_error)?;
    check!(eq; rows.errors, 2);
    let silent = census_crawl::AdapterReport::new("athleticnet", "performances");
    let rows = super::source_rows("test_slug", 5, &silent).map_err(sdk_error)?;
    check!(eq; rows.errors, 0);
    Ok(())
}
