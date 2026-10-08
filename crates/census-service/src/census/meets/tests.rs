use super::*;

fn row(fragment: &str, jurisdiction: UsJurisdiction, meet_id: &str) -> SourceMeetRef {
    SourceMeetRef {
        id: SourceMeetRef::row_id(SOURCE, meet_id),
        source: SOURCE.to_string(),
        source_meet_id: meet_id.to_string(),
        jurisdiction,
        season: "cc".to_string(),
        year: 2026,
        name: fragment.to_string(),
        date: None,
        venue: String::new(),
        results_url: format!(
            "https://{}.milesplit.com/meets/{meet_id}/results",
            jurisdiction.code().to_ascii_lowercase()
        ),
        observed_on: "2026-09-22".to_string(),
    }
}
fn row_with_year(
    fragment: &str,
    jurisdiction: UsJurisdiction,
    meet_id: &str,
    year: u16,
) -> SourceMeetRef {
    SourceMeetRef {
        id: SourceMeetRef::row_id(SOURCE, meet_id),
        source: SOURCE.to_string(),
        source_meet_id: meet_id.to_string(),
        jurisdiction,
        season: "cc".to_string(),
        year,
        name: fragment.to_string(),
        date: None,
        venue: String::new(),
        results_url: format!(
            "https://{}.milesplit.com/meets/{meet_id}/results",
            jurisdiction.code().to_ascii_lowercase()
        ),
        observed_on: "2026-09-22".to_string(),
    }
}

#[test]
fn meets_are_selected_in_a_reproducible_order() {
    let rows = vec![
        row("c", UsJurisdiction::Ohio, "900"),
        row("a", UsJurisdiction::Ohio, "100"),
        row("b", UsJurisdiction::Michigan, "100"),
    ];
    let selected = select_meets(
        rows,
        &[UsJurisdiction::Ohio, UsJurisdiction::Michigan],
        SeasonScope::All,
        None,
    );
    let order: Vec<(&str, &str)> = selected
        .iter()
        .map(|meet| (meet.jurisdiction.code(), meet.source_meet_id.as_str()))
        .collect();
    assert_eq!(order, [("MI", "100"), ("OH", "100"), ("OH", "900")]);
}

#[test]
fn a_limit_is_applied_per_state() {
    let rows = vec![
        row("a", UsJurisdiction::Ohio, "1"),
        row("b", UsJurisdiction::Ohio, "2"),
        row("c", UsJurisdiction::Ohio, "3"),
        row("d", UsJurisdiction::Michigan, "9"),
    ];
    let selected = select_meets(
        rows,
        &[UsJurisdiction::Ohio, UsJurisdiction::Michigan],
        SeasonScope::All,
        Some(2),
    );
    let ohio = selected
        .iter()
        .filter(|meet| meet.jurisdiction == UsJurisdiction::Ohio)
        .count();
    let michigan = selected
        .iter()
        .filter(|meet| meet.jurisdiction == UsJurisdiction::Michigan)
        .count();
    assert_eq!((ohio, michigan), (2, 1));
}

#[test]
fn a_state_with_no_stored_meets_selects_nothing() {
    let rows = vec![row("a", UsJurisdiction::Ohio, "1")];
    let selected = select_meets(rows, &[UsJurisdiction::Kansas], SeasonScope::All, None);
    assert!(selected.is_empty());
}

#[test]
fn season_scope_filters_by_year_before_jurisdiction_and_limit() {
    let rows: Vec<SourceMeetRef> = vec![
        row_with_year("wi-2025-a", UsJurisdiction::Wisconsin, "w10", 2025),
        row_with_year("wi-2025-b", UsJurisdiction::Wisconsin, "w11", 2025),
        row_with_year("wi-2026-a", UsJurisdiction::Wisconsin, "w20", 2026),
        row_with_year("oh-2026-a", UsJurisdiction::Ohio, "o10", 2026),
    ];

    let selected = select_meets(
        rows.clone(),
        &[UsJurisdiction::Wisconsin],
        SeasonScope::Year(2026),
        None,
    );
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].source_meet_id, "w20");
    assert_eq!(selected[0].year, 2026);

    let selected = select_meets(
        rows.clone(),
        &[UsJurisdiction::Wisconsin],
        SeasonScope::Year(2025),
        None,
    );
    assert_eq!(selected.len(), 2);
    let ids: Vec<&str> = selected.iter().map(|r| r.source_meet_id.as_str()).collect();
    assert_eq!(ids, vec!["w10", "w11"]);

    let selected = select_meets(
        rows.clone(),
        &[UsJurisdiction::Wisconsin],
        SeasonScope::Year(2024),
        None,
    );
    assert!(selected.is_empty());
}

#[test]
fn all_seasons_selects_every_year() {
    let rows: Vec<SourceMeetRef> = vec![
        row_with_year("wi-2025-a", UsJurisdiction::Wisconsin, "w10", 2025),
        row_with_year("wi-2025-b", UsJurisdiction::Wisconsin, "w11", 2025),
        row_with_year("wi-2026-a", UsJurisdiction::Wisconsin, "w20", 2026),
        row_with_year("oh-2026-a", UsJurisdiction::Ohio, "o10", 2026),
    ];

    let selected = select_meets(
        rows.clone(),
        &[UsJurisdiction::Wisconsin],
        SeasonScope::All,
        None,
    );
    assert_eq!(selected.len(), 3);

    let selected = select_meets(rows.clone(), &[], SeasonScope::All, None);
    assert_eq!(selected.len(), 4);
}

#[test]
fn season_scope_applies_before_limit() {
    let rows: Vec<SourceMeetRef> = vec![
        row_with_year("wi-2025-a", UsJurisdiction::Wisconsin, "w10", 2025),
        row_with_year("wi-2025-b", UsJurisdiction::Wisconsin, "w11", 2025),
        row_with_year("wi-2025-c", UsJurisdiction::Wisconsin, "w12", 2025),
        row_with_year("wi-2026-a", UsJurisdiction::Wisconsin, "w20", 2026),
    ];

    let selected = select_meets(
        rows.clone(),
        &[UsJurisdiction::Wisconsin],
        SeasonScope::Year(2025),
        Some(1),
    );
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].source_meet_id, "w10");

    let selected = select_meets(
        rows,
        &[UsJurisdiction::Wisconsin],
        SeasonScope::All,
        Some(1),
    );
    assert_eq!(selected.len(), 1);
}

#[path = "../../../tests/common/capture_cache.rs"]
mod cache;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tail {
    Zero,
    Empty,
    More,
    Repeated,
}

fn walk_fixture(tail: Tail) -> TestResult<(tempfile::TempDir, Store, MeetCensus)> {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path().join("store"))?;
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        std::time::Duration::ZERO,
        std::collections::HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true);
    let site = Site::for_jurisdiction(UsJurisdiction::Ohio);
    for season in Season::ALL {
        let pages = if season != Season::CrossCountry || tail == Tail::Zero {
            1
        } else if tail == Tail::Repeated {
            3
        } else {
            400
        };
        for page in 1..=pages {
            let id = if tail == Tail::Repeated && page == 3 {
                2
            } else {
                page
            };
            let row = if season != Season::CrossCountry
                || tail == Tail::Zero
                || (tail == Tail::Empty && page == 400)
            {
                String::new()
            } else {
                format!(
                    r#"<li class="meet-row" data-meet-id="{id}" data-filter-text="invite"><span class="meet-row__day">Sep 19</span><a class="meet-row__name" href="https://oh.milesplit.com/meets/{id}-invite/results">Invite {id}</a></li>"#
                )
            };
            let next = if season == Season::CrossCountry && (page < pages || tail == Tail::More) {
                r#"<a rel="next">Next</a>"#
            } else {
                ""
            };
            let body = format!(
                r#"<meta name="application-name" content="MileSplit"><section class="meet-month" data-month="2026-09">{row}</section>{next}"#
            );
            cache::seed(
                fetcher.cache_dir(),
                &site.results_url(season, 2026, page),
                body.as_bytes(),
                "2026-09-22T12:00:00Z",
                &[],
            )?;
        }
    }
    let options = census_crawl::net::FetchOptions::default();
    let request = MeetWalkRequest::new(UsJurisdiction::Ohio, 2026, "2026-09-22", &options);
    let census = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(collect_state_meets(&fetcher, &store, &request, None))?;
    Ok((root, store, census))
}

fn retained_prefix(store: &Store, last: u32) -> TestResult {
    let rows: Vec<SourceMeetRef> = store.scan(census_store::Table::SourceMeets)?;
    check!(eq; rows.len(), usize::try_from(last)?);
    let observed: std::collections::BTreeSet<_> = rows
        .into_iter()
        .map(|row| {
            (
                row.id,
                row.source_meet_id,
                row.jurisdiction,
                row.year,
                row.season,
                row.observed_on,
                row.date,
            )
        })
        .collect();
    let expected = (1..=last)
        .map(|id| {
            (
                format!("milesplit:{id}"),
                id.to_string(),
                UsJurisdiction::Ohio,
                2026,
                "cc".to_string(),
                "2026-09-22".to_string(),
                Some("2026-09-19".to_string()),
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    check!(eq; observed, expected);
    Ok(())
}

#[test]
fn empty_page_400_exhausts_the_index_without_losing_its_admitted_prefix() -> TestResult {
    let (_root, store, census) = walk_fixture(Tail::Empty)?;
    retained_prefix(&store, 399)?;
    check!(census.is_terminal());
    check!(eq; (census.pages, census.rows, census.truncated, census.repeated), (402, 399, 0, 0));
    let source = census.sources.first().ok_or("missing measured source")?;
    check!(eq; source.disposition, census_crawl::CollectionDisposition::Complete);
    check!(eq; source.unresolved, Some(census_crawl::UnresolvedCounters { rows: 0, labels: 0 }));
    let cursor = store
        .journal_payload(
            &meets_phase(UsJurisdiction::Ohio, Season::CrossCountry, 2026),
            "cursor",
        )?
        .ok_or("missing cursor")?;
    check!(eq; (cursor["next_page"].as_u64(), cursor["disposition"].as_str()), (Some(400), Some("exhausted")));
    Ok(())
}

#[test]
fn nonempty_page_400_retains_page_401_as_explicit_debt() -> TestResult {
    let (_root, store, census) = walk_fixture(Tail::More)?;
    retained_prefix(&store, 400)?;
    check!(!census.is_terminal());
    let source = census.sources.first().ok_or("missing measured source")?;
    check!(eq; source.unresolved, None);
    check!(eq; source.disposition, census_crawl::CollectionDisposition::Partial);
    check!(eq; source.unfinished, vec![Site::for_jurisdiction(UsJurisdiction::Ohio).results_url(Season::CrossCountry, 2026, 401)]);
    let cursor = store
        .journal_payload(
            &meets_phase(UsJurisdiction::Ohio, Season::CrossCountry, 2026),
            "cursor",
        )?
        .ok_or("missing cursor")?;
    check!(eq; (cursor["next_page"].as_u64(), cursor["disposition"].as_str()), (Some(401), Some("partial")));
    Ok(())
}

#[test]
fn repeated_published_page_is_quarantined_not_successful_exhaustion() -> TestResult {
    let (_root, store, census) = walk_fixture(Tail::Repeated)?;
    retained_prefix(&store, 2)?;
    check!(!census.is_terminal());
    check!(eq; (census.rows, census.repeated), (2, 1));
    let source = census.sources.first().ok_or("missing measured source")?;
    check!(eq; source.unresolved, None);
    check!(eq; source.disposition, census_crawl::CollectionDisposition::Partial);
    check!(eq; source.unfinished, vec![Site::for_jurisdiction(UsJurisdiction::Ohio).results_url(Season::CrossCountry, 2026, 3)]);
    let cursor = store
        .journal_payload(
            &meets_phase(UsJurisdiction::Ohio, Season::CrossCountry, 2026),
            "cursor",
        )?
        .ok_or("missing cursor")?;
    check!(eq; (cursor["next_page"].as_u64(), cursor["disposition"].as_str()), (Some(3), Some("quarantined")));
    Ok(())
}

#[test]
fn exhausted_empty_seasons_are_measured_zero_not_unattempted() -> TestResult {
    let (_root, store, census) = walk_fixture(Tail::Zero)?;
    retained_prefix(&store, 0)?;
    check!(eq; (census.pages, census.rows), (3, 0));
    check!(census.is_terminal());
    let source = census.sources.first().ok_or("missing measured source")?;
    check!(eq; source.disposition, census_crawl::CollectionDisposition::Complete);
    check!(eq; source.unresolved, Some(census_crawl::UnresolvedCounters { rows: 0, labels: 0 }));
    Ok(())
}
