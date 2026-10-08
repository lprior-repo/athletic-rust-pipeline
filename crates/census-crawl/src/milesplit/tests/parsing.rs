use super::*;

#[test]
fn dc_legacy_fixture_parses_result_file_entries() -> TestResult {
    const DC_LEGACY: &str =
        include_str!("../../../tests/fixtures/milesplit/dc_meet_735841_results_legacy.html");
    let files = parse_meet_result_files(
        "https://www.milesplit.com/meets/735841-stancs-home-meet-1-2026/results",
        DC_LEGACY,
    )?;
    check!(eq; files.len(), 2, "two per-file result entries");
    check!(eq; files[0].id, 1257095);
    check!(eq; files[0].name, "Varsity Boys Results");
    check!(eq; files[0].is_meet_pro, 0);
    check!(eq; files[1].id, 1257096);
    check!(eq; files[1].name, "Varsity Girls Results");
    check!(eq; files[1].is_meet_pro, 0);
    Ok(())
}

#[test]
fn legacy_select_with_only_all_is_empty() -> TestResult {
    let html = r#"
<select id="ddResultsPage">
    <option value="https://www.milesplit.com/meets/999999-x/results">All</option>
</select>
"#;
    let files = parse_meet_result_files("https://www.milesplit.com/meets/999999-x/results", html)?;
    check!(files.is_empty(), "only-All legacy select yields empty list");
    Ok(())
}

#[test]
fn an_inline_results_page_is_its_own_one_result_set() -> TestResult {
    const DC_INLINE: &str =
        include_str!("../../../tests/fixtures/milesplit/dc_meet_764735_results_inline.html");
    const DC_INLINE_URL: &str =
        "https://www.milesplit.com/meets/764735-dc10-track-fest-hosted-by-light-horse-track-club-2026/results";

    let files = parse_meet_result_files(DC_INLINE_URL, DC_INLINE)?;
    check!(eq; files.len(), 1, "an inline page publishes one result set");
    check!(files[0].inline, "the page is the result set");
    check!(eq; files[0].id, 0, "an inline set has no published file id");

    let url = files[0].raw_url(DC_INLINE_URL);
    check!(eq; url, DC_INLINE_URL, "the rows are the page itself");

    let reference = ResultSetRef::parse_with_jurisdiction(&url, UsJurisdiction::DistrictOfColumbia)
        .ok_or("the inline page is a result set the run can address")?;
    check!(eq; reference.meet_id, "764735");
    check!(eq; reference.rsid, "0");

    let page = parse_raw(DC_INLINE, &url)?;
    check!(
        page.meet.rows_parsed > 0,
        "the capture publishes rows, not an empty block: {} line(s) skipped",
        page.skipped.len()
    );
    Ok(())
}

#[test]
fn www_host_is_rejected_by_parse_but_accepted_by_parse_with_jurisdiction() -> TestResult {
    let www_url =
        "https://www.milesplit.com/meets/735841-stancs-home-meet-1-2026/results/1257095/raw";
    check!(
        ResultSetRef::parse(www_url).is_none(),
        "www host rejected by plain parse"
    );
    let ref_with_jur =
        ResultSetRef::parse_with_jurisdiction(www_url, UsJurisdiction::DistrictOfColumbia);
    let reference = ref_with_jur.ok_or("www host accepted with explicit jurisdiction")?;
    check!(eq; reference.meet_id, "735841");
    check!(eq; reference.rsid, "1257095");
    check!(eq; reference.site.code(), "DC");
    check!(eq; reference.url, www_url);
    Ok(())
}

#[test]
fn unknown_host_is_rejected_by_both_parse_methods() {
    let fake_url = "https://fake.milesplit.com/meets/123456-x/results/789/raw";
    assert!(ResultSetRef::parse(fake_url).is_none());
    assert!(ResultSetRef::parse_with_jurisdiction(fake_url, UsJurisdiction::Ohio).is_none());
}

#[test]
fn neither_template_is_a_schema_error() -> TestResult {
    let html = r#"
<html>
<head><title>Results</title></head>
<body>
<p>No file list here at all.</p>
</body>
</html>
"#;
    let error =
        match parse_meet_result_files("https://www.milesplit.com/meets/999999-x/results", html) {
            Err(error) => error,
            Ok(_) => return Err("neither template should be a schema error".into()),
        };
    check!(
        matches!(error, CrawlError::Schema { .. }),
        "expected schema error: {error:?}"
    );
    Ok(())
}

#[test]
fn parses_a_state_results_index_into_requestable_meets() -> TestResult {
    let meets = parse_meet_index(RESULTS_INDEX)?;
    check!(eq;
        meets.len(),
        50,
        "the captured page publishes fifty meet rows"
    );
    let first = &meets[0];
    check!(eq; first.meet_id, "770621");
    check!(eq; first.name, "Beaver Eastern Invite");
    check!(eq; first.venue, "Beaver, OH");
    check!(eq;
        first.results_url,
        "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results"
    );
    check!(eq;
        first.meet_url(),
        "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026"
    );
    check!(eq; first.date.as_deref(), Some("2026-09-19"));
    check!(meets.iter().all(|meet| !meet.meet_id.is_empty()));
    check!(
        meets.iter().all(|meet| meet.date.is_some()),
        "every row of the capture sits under a month bucket"
    );
    check!(
        has_next_page(RESULTS_INDEX),
        "page one publishes a next page"
    );
    Ok(())
}

#[test]
fn a_meet_row_without_an_id_is_not_a_meet() -> TestResult {
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
    match parse_meet_index(html) {
        Err(CrawlError::Schema { url, .. }) => check!(eq; url, "meet index"),
        outcome => {
            return Err(
                format!("missing required provider id must refuse the index: {outcome:?}").into(),
            )
        }
    }
    Ok(())
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

#[test]
fn captured_nc_wrapped_round_keeps_its_event_and_finisher() -> TestResult {
    let block = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/sources/milesplit-national/samples/raw-nc-684812-rs1283641.txt"
    ));
    let html = format!(
        "<script type=\"application/ld+json\">{{\"name\":\"PR Running Camp\",\"startDate\":\"2025-03-08\",\"sport\":\"Track\"}}</script><pre>{block}</pre>"
    );
    let page = parse_raw(
        &html,
        "https://nc.milesplit.com/meets/684812/results/1283641/raw",
    )?;
    let event = page
        .meet
        .events
        .iter()
        .find(|event| event.rows.iter().any(|row| row.name == "FERGUSON, Michael"))
        .ok_or("published finisher retained")?;
    check!(eq; event.kind, EventKind::from_source_label("3200M"));
    check!(eq; event.gender, Gender::Boys);
    check!(eq; event.round.as_deref(), Some("Finals"));
    check!(eq; page.meet.events.len(), 1);
    check!(eq; event.label, "PR Running Camp Boys 3200M Finals");
    check!(eq; event.rows.len(), 214);
    for (name, school, grade, mark) in [
        ("WHARTON, Elijah", "Davidson Academy", 11, "DNF"),
        ("WILLCOX, Jack", "Heathwood Hall", 11, "DNF"),
        ("JENKINS, Grady", "Texas", 12, "NT"),
        ("TEMPLETON, John", "Forsyth Country Day", 12, "NT"),
    ] {
        let status_row = event
            .rows
            .iter()
            .find(|row| row.name == name)
            .ok_or("published status row")?;
        check!(eq; status_row.place, None);
        check!(eq; status_row.school, school);
        check!(eq; status_row.grade.map(|grade| grade.get()), Some(grade));
        check!(eq; status_row.mark, Mark::Raw(mark.to_string()));
        check!(eq; status_row.heat.as_deref(), Some("7"));
    }
    let row = event
        .rows
        .iter()
        .find(|row| row.name == "FERGUSON, Michael")
        .ok_or("published finisher")?;
    check!(eq; row.school, "North Buncombe");
    check!(eq; row.grade.map(|grade| grade.get()), Some(12));
    check!(eq;
        row.mark,
        crate::hytek::parse_time("8:44.73")
            .map(Mark::TimeSeconds)
            .ok_or("published mark")?
    );
    check!(eq; row.heat.as_deref(), Some("8"));
    check!(
        page.skipped.is_empty(),
        "located grade tokens are not whole-row rejections: {:?}",
        page.skipped
    );
    check!(
        !page.grade_issues.is_empty(),
        "the capture publishes eighth graders"
    );
    check!(
        page.grade_issues.iter().all(|issue| {
            issue.kind == RawGradeIssueKind::OutsideHighSchool && issue.raw_token == "8"
        }),
        "{:?}",
        page.grade_issues
    );
    let located = page
        .grade_issues
        .iter()
        .find(|issue| issue.raw_token == "8")
        .ok_or("the capture's eighth-grade row")?;
    let start = located.row.byte_offset;
    let end = start.saturating_add(located.row.byte_length);
    let published = html
        .get(start..end)
        .ok_or("the locator resolves the captured HTML bytes")?;
    check!(
        published.contains("SURFACE, Luke"),
        "published: {published:?}"
    );
    let eighth = event
        .rows
        .iter()
        .find(|row| row.name == "SURFACE, Luke")
        .ok_or("an eighth grader is a published row")?;
    check!(eq; eighth.grade, None);
    check!(eq; eighth.school, "North Raleigh Christ");
    Ok(())
}
