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
            let Json(outcome) = collect_fragment(
                Arc::clone(&store),
                fetcher,
                scope,
                ("coach_directories".to_string(), 0),
                "unselected-index-test".to_string(),
            )
            .await
            .map_err(crate::restate_services::tests::sdk_error)?;
            check!(eq; outcome.census, MeetCensus::default());
            check!(eq; store.walk_table(census_store::Table::SourceMeets)?.rows, 0);
            check!(eq; outcome.spool.fragments, 0);
            Ok(())
        })
}

#[test]
fn large_diagnostics_travel_as_bounded_references_without_losing_details() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let census = MeetCensus {
        rows: 1,
        sources: vec![MeetSourceRows {
            slug: "wiaa_results".into(),
            rows: 1,
            disposition: CollectionDisposition::Partial,
            errors: vec!["unresolved published label ".repeat(40_000)],
            notes: vec!["retained diagnostic detail ".repeat(40_000)],
            unfinished: vec!["artifact incomplete".into()],
            withheld: Some(0),
            unresolved: Some(census_crawl::UnresolvedCounters { rows: 1, labels: 1 }),
        }],
        ..MeetCensus::default()
    };
    check!(serde_json::to_vec(&census)?.len() > 4096);
    let bounded = diagnostics::bounded_census(&store, census.clone(), "diagnostics-test/0", 0)
        .map_err(crate::restate_services::tests::sdk_error)?;
    check!(serde_json::to_vec(&bounded)?.len() <= 4096);
    check!(!bounded.is_terminal());
    let original: MeetCensus = serde_json::from_value(
        store
            .journal_payload(diagnostics::PHASE, "diagnostics-test/0")?
            .ok_or("durable diagnostic record")?,
    )?;
    check!(eq; original, census);
    let combined = absorb_fragment("wiaa_results", bounded.clone(), bounded)
        .map_err(crate::restate_services::tests::sdk_error)?;
    check!(serde_json::to_vec(&combined)?.len() <= 4096);
    check!(eq; combined.rows, 2);
    Ok(())
}

#[test]
fn athleticnet_historical_discovery_has_a_durable_arm() -> TestResult {
    check!(
        arm_for("athleticnet").is_some(),
        "Athletic.net historical discovery must register even when its frontier is refused"
    );
    Ok(())
}

#[test]
fn missing_producer_frontiers_are_durable_typed_refusals() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let store = Arc::new(Store::open(root.path().join("store"))?);
            let fetcher = Arc::new(
                Fetcher::new(
                    store.http_cache_dir(),
                    None,
                    std::time::Duration::ZERO,
                    std::collections::HashMap::new(),
                    Vec::new(),
                )?
                .with_offline(true),
            );
            let scope = HistoricalStageScope {
                jurisdiction: census_domain::UsJurisdiction::Wisconsin,
                year: 2024,
                refresh: false,
                window: HistoryWindow::new(2023, 2026, "2026-10-07")?,
                observed_on: chrono::NaiveDate::parse_from_str("2026-10-08", "%Y-%m-%d")?,
            };
            for slug in [
                "athleticnet",
                "athleticlive",
                "athleticlive_athletes",
                "tfrrs",
            ] {
                let identity = format!("prerequisite-test/{slug}");
                let Json(outcome) = collect_fragment(
                    Arc::clone(&store),
                    Arc::clone(&fetcher),
                    scope,
                    (slug.into(), 0),
                    identity.clone(),
                )
                .await
                .map_err(crate::restate_services::tests::sdk_error)?;
                let source = outcome
                    .census
                    .sources
                    .first()
                    .ok_or("missing selected source")?;
                check!(eq; source.slug, slug);
                check!(eq; source.disposition, CollectionDisposition::Blocked);
                check!(eq; source.rows, 0);
                check!(!source.notes.is_empty());
                check!(!source.unfinished.is_empty());
                check!(eq; source.unresolved, None);
                check!(eq; outcome.next, None);
                check!(eq; outcome.spool.fragments, 0);
                check!(!outcome.census.is_terminal());
                check!(serde_json::to_vec(&outcome)?.len() <= 4096);
                let saved = store
                    .journal_payload("meet_fragment_outcomes_v1", &identity)?
                    .ok_or("refusal must be durable")?;
                check!(eq; saved, serde_json::to_value(&outcome)?);
                let Json(replayed) = collect_fragment(
                    Arc::clone(&store),
                    Arc::clone(&fetcher),
                    scope,
                    (slug.into(), 0),
                    identity,
                )
                .await
                .map_err(crate::restate_services::tests::sdk_error)?;
                check!(eq; serde_json::to_value(replayed)?, saved);
            }
            check!(eq; fetcher.stats().await.physical_requests(), 0);
            check!(eq; store.walk_table(census_store::Table::SourceMeets)?.rows, 0);
            Ok(())
        })
}

#[tokio::test]
async fn every_applicable_meet_family_is_armed_or_has_a_durable_named_refusal() -> TestResult {
    use crate::restate_services::plan::{plan, BrowserLaneState, UnitDisposition};
    use crate::restate_services::wire::SourcePlan;
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path().join("store"))?);
    let fetcher = Arc::new(
        Fetcher::new(
            store.http_cache_dir(),
            None,
            std::time::Duration::ZERO,
            std::collections::HashMap::new(),
            Vec::new(),
        )?
        .with_offline(true),
    );
    for jurisdiction in census_domain::UsJurisdiction::CENSUS_SCOPE {
        let dispositions = plan(jurisdiction, BrowserLaneState::Configured);
        let recorded = SourcePlan::of(&dispositions, String::new());
        for source in census_crawl::applicability::applicable_sources(jurisdiction) {
            if !required(source) {
                continue;
            }
            let disposition = dispositions
                .iter()
                .find(|unit| unit.slug() == source.slug)
                .ok_or("applicable discovery obligation missing from plan")?;
            if let UnitDisposition::Refused(refusal) = disposition {
                check!(!refusal.reason.is_empty());
                check!(eq; refusal.kind, crate::restate_services::plan::refusal_kind(source.slug), "the refusal kind comes from the shared classifier");
                check!(recorded
                    .refused
                    .iter()
                    .any(|row| row.slug == source.slug && row.reason == refusal.reason));
                check!(jobs::assert_some_stage_arms(source.slug).is_err());
            } else {
                jobs::assert_some_stage_arms(source.slug)
                    .map_err(crate::restate_services::tests::sdk_error)?;
            }
            if arm_for(source.slug).is_some() {
                continue;
            }
            let identity = format!("family-refusal/{}/{}", jurisdiction.code(), source.slug);
            let scope = HistoricalStageScope {
                jurisdiction,
                year: 2026,
                refresh: false,
                window: HistoryWindow::new(2023, 2026, "2026-10-07")?,
                observed_on: chrono::NaiveDate::parse_from_str("2026-10-08", "%Y-%m-%d")?,
            };
            let Json(outcome) = collect_fragment(
                Arc::clone(&store),
                Arc::clone(&fetcher),
                scope,
                (source.slug.into(), 0),
                identity.clone(),
            )
            .await
            .map_err(crate::restate_services::tests::sdk_error)?;
            let row = outcome
                .census
                .sources
                .first()
                .ok_or("unarmed source refusal")?;
            check!(eq; row.slug, source.slug);
            check!(eq; row.disposition, CollectionDisposition::Blocked);
            check!(!row.unfinished.is_empty());
            check!(eq; row.rows, 0);
            check!(!outcome.census.is_terminal());
            check!(store
                .journal_payload("meet_fragment_outcomes_v1", &identity)?
                .is_some());
        }
    }
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}
