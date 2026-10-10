use super::*;
use crate::restate_services::wire::HistoryWindow;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn clean_counts_do_not_certify_an_unknown_meet_frontier() -> TestResult {
    let mut report = AdapterReport::new("wiaa_results", "results");
    report.rows = 12;
    report.notes = vec!["held: source discovery is incomplete".to_string()];
    let census =
        report_census("wiaa_results", report).map_err(crate::restate_services::tests::sdk_error)?;
    check!(eq; census.rows, 12);
    check!(eq; census.sources.first().ok_or("source report")?.disposition, CollectionDisposition::Unknown);
    check!(eq; census.sources.first().ok_or("source report")?.notes,
        vec!["held: source discovery is incomplete"]);
    check!(eq; census.sources.first().ok_or("source report")?.unresolved, None);
    check!(!census.is_terminal());
    Ok(())
}

#[test]
fn rejected_and_unresolved_meet_rows_keep_authoritative_unfinished_locators() -> TestResult {
    let mut report = AdapterReport::new("wiaa_results", "results");
    report.rows = 7;
    report.disposition = CollectionDisposition::Partial;
    report.rejections = 1;
    report.unresolved = Some(census_crawl::UnresolvedCounters { rows: 2, labels: 1 });
    report.unfinished = vec!["https://www.wiaawi.org/Results/Track/2024/d1.htm#row-8".to_string()];
    let census =
        report_census("wiaa_results", report).map_err(crate::restate_services::tests::sdk_error)?;
    let source = census.sources.first().ok_or("source report")?;
    check!(eq; source.rows, 7);
    check!(eq; source.unfinished, vec!["https://www.wiaawi.org/Results/Track/2024/d1.htm#row-8"]);
    check!(eq; source.errors.len(), 1);
    check!(eq; source.withheld, Some(1));
    check!(eq; source.unresolved, Some(census_crawl::UnresolvedCounters { rows: 2, labels: 1 }));
    check!(!census.is_terminal());
    Ok(())
}

#[test]
fn every_meet_family_slug_maps_to_exactly_one_arm() -> TestResult {
    use crate::restate_services::plan::Dispatch;
    let mut slugs: Vec<&str> = MEETS_ARMS.iter().map(|(slug, _)| *slug).collect();
    let mut sorted = slugs.clone();
    sorted.sort_unstable();
    slugs.sort_unstable();
    slugs.dedup();
    check!(
        eq;
        slugs.len(),
        sorted.len(),
        "a meet slug must not carry conflicting arms: {sorted:?}"
    );
    for (slug, arm) in MEETS_ARMS {
        check!(eq; arm_for(slug), Some(*arm), "slug {slug}");
        check!(eq; Dispatch::of(slug), Dispatch::Wired, "the plan wires {slug}");
    }
    Ok(())
}

#[test]
fn an_unwired_meet_family_slug_refuses_by_name() -> TestResult {
    use crate::restate_services::plan::Dispatch;
    check!(eq; arm_for("athleticlive"), None);
    check!(
        eq;
        Dispatch::of("athleticlive"),
        Dispatch::Unwired,
        "the plan owes the AthleticLIVE artifact family"
    );
    let refusal = jobs::assert_some_stage_arms("athleticlive")
        .err()
        .ok_or("an applicable meet family without an arm must fail closed")?;
    check!(
        format!("{refusal:?}").contains("athleticlive"),
        "the refusal names the slug: {refusal:?}"
    );
    Ok(())
}

#[test]
fn unselected_milesplit_never_opens_its_index_frontier() -> TestResult {
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
            let scope = HistoricalStageScope {
                jurisdiction: census_domain::UsJurisdiction::Wisconsin,
                year: 2024,
                refresh: false,
                window: HistoryWindow::new(2023, 2026, "2026-10-07")?,
                observed_on: chrono::NaiveDate::parse_from_str("2026-10-08", "%Y-%m-%d")?,
            };
            let selected = SourceSelection::parse(&["coach_directories".to_string()])
                .map_err(crate::restate_services::tests::sdk_error)?;
            let Json(outcome) = meets_stage(Arc::clone(&store), fetcher, scope, selected)
                .await
                .map_err(crate::restate_services::tests::sdk_error)?;
            check!(eq; outcome.census, MeetCensus::default());
            check!(eq; store.walk_table(census_store::Table::SourceMeets)?.rows, 0);
            check!(eq; outcome.recorded.len(), 0);
            Ok(())
        })
}
