use super::*;

#[test]
fn dc_legacy_fixture_parses_result_file_entries() {
    const DC_LEGACY: &str =
        include_str!("../../../tests/fixtures/milesplit/dc_meet_735841_results_legacy.html");
    let files = parse_meet_result_files(
        "https://www.milesplit.com/meets/735841-stancs-home-meet-1-2026/results",
        DC_LEGACY,
    )
    .unwrap();
    assert_eq!(files.len(), 2, "two per-file result entries");
    assert_eq!(files[0].id, 1257095);
    assert_eq!(files[0].name, "Varsity Boys Results");
    assert_eq!(files[0].is_meet_pro, 0);
    assert_eq!(files[1].id, 1257096);
    assert_eq!(files[1].name, "Varsity Girls Results");
    assert_eq!(files[1].is_meet_pro, 0);
}

#[test]
fn legacy_select_with_only_all_is_empty() {
    let html = r#"
<select id="ddResultsPage">
    <option value="https://www.milesplit.com/meets/999999-x/results">All</option>
</select>
"#;
    let files =
        parse_meet_result_files("https://www.milesplit.com/meets/999999-x/results", html).unwrap();
    assert!(files.is_empty(), "only-All legacy select yields empty list");
}

#[test]
fn an_inline_results_page_is_its_own_one_result_set() {
    const DC_INLINE: &str =
        include_str!("../../../tests/fixtures/milesplit/dc_meet_764735_results_inline.html");
    const DC_INLINE_URL: &str =
        "https://www.milesplit.com/meets/764735-dc10-track-fest-hosted-by-light-horse-track-club-2026/results";

    let files = parse_meet_result_files(DC_INLINE_URL, DC_INLINE).unwrap();
    assert_eq!(files.len(), 1, "an inline page publishes one result set");
    assert!(files[0].inline, "the page is the result set");
    assert_eq!(files[0].id, 0, "an inline set has no published file id");

    let url = files[0].raw_url(DC_INLINE_URL);
    assert_eq!(url, DC_INLINE_URL, "the rows are the page itself");

    let reference = ResultSetRef::parse_with_jurisdiction(&url, UsJurisdiction::DistrictOfColumbia)
        .expect("the inline page is a result set the run can address");
    assert_eq!(reference.meet_id, "764735");
    assert_eq!(reference.rsid, "0");

    let page = parse_raw(DC_INLINE, &url).expect("the page carries a raw body");
    assert!(
        page.meet.rows_parsed > 0,
        "the capture publishes rows, not an empty block: {} line(s) skipped",
        page.skipped.len()
    );
}

#[test]
fn www_host_is_rejected_by_parse_but_accepted_by_parse_with_jurisdiction() {
    let www_url =
        "https://www.milesplit.com/meets/735841-stancs-home-meet-1-2026/results/1257095/raw";
    assert!(
        ResultSetRef::parse(www_url).is_none(),
        "www host rejected by plain parse"
    );
    let ref_with_jur =
        ResultSetRef::parse_with_jurisdiction(www_url, UsJurisdiction::DistrictOfColumbia);
    let reference = ref_with_jur.expect("www host accepted with explicit jurisdiction");
    assert_eq!(reference.meet_id, "735841");
    assert_eq!(reference.rsid, "1257095");
    assert_eq!(reference.site.code(), "DC");
    assert_eq!(reference.url, www_url);
}

#[test]
fn unknown_host_is_rejected_by_both_parse_methods() {
    let fake_url = "https://fake.milesplit.com/meets/123456-x/results/789/raw";
    assert!(ResultSetRef::parse(fake_url).is_none());
    assert!(ResultSetRef::parse_with_jurisdiction(fake_url, UsJurisdiction::Ohio).is_none());
}

#[test]
fn neither_template_is_a_schema_error() {
    let html = r#"
<html>
<head><title>Results</title></head>
<body>
<p>No file list here at all.</p>
</body>
</html>
"#;
    let error = parse_meet_result_files("https://www.milesplit.com/meets/999999-x/results", html)
        .expect_err("neither template should be a schema error");
    assert!(
        matches!(error, CrawlError::Schema { .. }),
        "expected schema error: {error:?}"
    );
}

#[test]
fn parses_a_state_results_index_into_requestable_meets() {
    let meets = parse_meet_index(RESULTS_INDEX).unwrap();
    assert_eq!(
        meets.len(),
        50,
        "the captured page publishes fifty meet rows"
    );
    let first = &meets[0];
    assert_eq!(first.meet_id, "770621");
    assert_eq!(first.name, "Beaver Eastern Invite");
    assert_eq!(first.venue, "Beaver, OH");
    assert_eq!(
        first.results_url,
        "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results"
    );
    assert_eq!(
        first.meet_url(),
        "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026"
    );
    assert_eq!(first.date.as_deref(), Some("2026-09-19"));
    assert!(meets.iter().all(|meet| !meet.meet_id.is_empty()));
    assert!(
        meets.iter().all(|meet| meet.date.is_some()),
        "every row of the capture sits under a month bucket"
    );
    assert!(
        has_next_page(RESULTS_INDEX),
        "page one publishes a next page"
    );
}

#[test]
fn a_meet_row_without_an_id_is_not_a_meet() {
    let html = r#"
<section class="meet-month" data-month="2026-09">
<li class="meet-row"
data-meet-id="770621"
data-filter-text="x"><span class="meet-row__day">Sep 19</span>
<a class="meet-row__name" href="https://oh.milesplit.com/meets/770621-x/results">X Invite</a></li>
<li class="meet-row"
data-filter-text="y"><span class="meet-row__day">Sep 20</span>
<a class="meet-row__name" href="https://oh.milesplit.com/meets/2-y/results">Y Invite</a></li>
<li class="meet-row"
data-meet-id="770622"
data-filter-text="z"><span class="meet-row__day">Sep 31</span>
<a class="meet-row__name" href="https://oh.milesplit.com/meets/770622-z/results">Z Invite</a></li>
</section>
"#;
    let meets = parse_meet_index(html).unwrap();
    assert_eq!(meets.len(), 2, "the row with no meet id is dropped");
    assert_eq!(meets[0].meet_id, "770621");
    assert_eq!(meets[0].date.as_deref(), Some("2026-09-19"));
    assert_eq!(
        meets[0].venue, "",
        "an absent venue is empty, never invented"
    );
    assert_eq!(meets[1].meet_id, "770622");
    assert_eq!(meets[1].date, None, "September has no 31st day");
    assert!(!has_next_page(html));
}

#[test]
fn the_results_index_url_is_the_published_query_shape() {
    let site = Site::for_jurisdiction(UsJurisdiction::Ohio);
    assert_eq!(
        site.results_url(Season::CrossCountry, 2026, 2),
        "https://oh.milesplit.com/results?season=cc&level=hs&year=2026&page=2"
    );
    assert_eq!(Season::ALL.map(Season::code), ["cc", "indoor", "outdoor"]);
}
