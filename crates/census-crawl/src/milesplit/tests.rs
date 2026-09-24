use super::*;
use crate::CrawlError;
use census_domain::model::{normalize_name, CanonicalSchool, EventKind, Grade, Mark};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;

use super::map::absorb_result_set;
use super::parse::parse_meet_result_files;
use super::results::{Accumulator, Stats};

const TEAMS: &str = include_str!("../../tests/fixtures/milesplit/wi_teams_index.html");
const ROSTER: &str = include_str!("../../tests/fixtures/milesplit/wi_roster_52649.html");
const RESULTS_INDEX: &str = include_str!("../../tests/fixtures/milesplit/oh_results_index.html");

#[test]
fn parses_team_index_rows() {
    let teams = parse_team_index(TEAMS).unwrap();
    assert_eq!(teams.len(), 40);
    assert_eq!(teams[0].id, "52649");
    assert_eq!(teams[0].name, "Abbotsford");
    assert_eq!(teams[0].city_state, "ABBOTSFORD, WI, USA");
}

#[test]
fn parses_roster_rows_with_grad_year_and_seasons() {
    let teams = parse_team_index(TEAMS).unwrap();
    let roster = parse_roster(ROSTER, teams[0].clone()).unwrap();
    assert_eq!(roster.athletes.len(), 25);
    let first = &roster.athletes[0];
    assert_eq!(first.name, "Julian Aguilera");
    assert_eq!(first.roster_name, "Aguilera, Julian");
    assert_eq!(first.grad_year, GradYear::CO2027);
    assert_eq!(first.gender, Gender::Boys);
    assert_eq!(first.athlete_id, "14399169");
    assert!(first
        .profile_url
        .ends_with("/athletes/14399169-julian-aguilera"));
    // This athlete's roster row carries no season flags (matched athlete, no imported results).
    assert!(first.sports().is_empty());
    let all_sports = roster
        .athletes
        .iter()
        .find(|athlete| athlete.roster_name.starts_with("Altamirano"))
        .expect("Altamirano row present in fixture");
    assert_eq!(
        all_sports.sports(),
        vec![Sport::IndoorTrack, Sport::OutdoorTrack, Sport::CrossCountry]
    );
}

#[test]
fn roster_entities_are_canonical_and_source_independent() {
    let teams = parse_team_index(TEAMS).unwrap();
    let roster = parse_roster(ROSTER, teams[0].clone()).unwrap();
    let site = Site::for_jurisdiction(UsJurisdiction::Wisconsin);
    let (school, athletes, teams_out) = roster_entities(
        &roster,
        SchoolYear::new(2026).expect("2026 is a season"),
        "2026-09-20",
        &site,
    );
    assert_eq!(school.name, "Abbotsford");
    assert_eq!(school.city.as_deref(), Some("Abbotsford"));
    assert!(!athletes.is_empty());
    assert!(teams_out.len() >= 2, "indoor/outdoor/XC team variants");
    let aguilera = athletes
        .iter()
        .find(|athlete| athlete.canonical_name == "Julian Aguilera")
        .unwrap();
    assert_eq!(aguilera.grad_year, GradYear::CO2027);
    // Grade observed on a 2026-27 roster is 12 for a 2027 graduate.
    let observation = aguilera.observed_grades.first().unwrap();
    assert_eq!(observation.grade.get(), 12);
    assert_eq!(observation.grad_year(), GradYear::CO2027);
    assert_eq!(
        aguilera.id,
        CanonicalAthlete::mint(
            &school.id,
            "Julian Aguilera",
            GradYear::CO2027,
            Gender::Boys
        )
    );
}

#[test]
fn malformed_html_fails_loudly() {
    assert!(parse_team_index("<html><body>no rows</body></html>").is_err());
    let teams = parse_team_index(TEAMS).unwrap();
    let roster = parse_roster("<html></html>", teams[0].clone()).unwrap();
    assert!(
        roster.athletes.is_empty(),
        "empty roster is data, not an error"
    );
}

// ---------------------------------------------------------------------------
// The state team index, the graded roster, and the `/raw` result set.
//
// Every fixture below is byte-identical to the lane capture it came from
// (`research/sources/milesplit-national/samples/`, checked with `cmp`): `teams-oh.html`,
// `roster-oh-mason.html` and `raw-oh-770621-rs1321880.txt`. Nothing here touches the network.
// ---------------------------------------------------------------------------

const OH_TEAMS: &str = include_str!("../../tests/fixtures/milesplit/oh_teams_index.html");
const OH_ROSTER: &str = include_str!("../../tests/fixtures/milesplit/oh_roster_10002_mason.html");
const OH_RAW: &str =
    include_str!("../../tests/fixtures/milesplit/oh_meet_770621_rs1321880_raw.html");
const OH_RAW_URL: &str =
    "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/raw";
const OH_MEET_RESULTS: &str =
    include_str!("../../tests/fixtures/milesplit/oh_meet_770621_results.html");
const OH_MEET_RESULTS_URL: &str =
    "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results";

/// One state's whole team index: 977 teams for OH in one body, measured as 977 team links by the
/// wave-2 sweep (`samples/teams-count-sweep.tsv`: `oh 200 32962 977`), whose 51 rows sum to the
/// 26,562 teams the index carries nationally. No pagination: the sweep read one body per state.
#[test]
fn team_index_row_count_pins_a_whole_state_in_one_body() {
    let teams = parse_team_index(OH_TEAMS).unwrap();
    assert_eq!(teams.len(), 977);
    let unique: std::collections::HashSet<&str> =
        teams.iter().map(|team| team.id.as_str()).collect();
    assert_eq!(unique.len(), teams.len(), "team ids are the index key");
    assert!(teams
        .iter()
        .all(|team| team.url.starts_with("https://oh.milesplit.com/teams/")));
    let mason = teams.iter().find(|team| team.id == "10002").expect("Mason");
    assert!(
        mason.city_state.contains("OH"),
        "city_state: {}",
        mason.city_state
    );
}

/// The graded roster: 319 rows, every row carrying `column-grad-year`, 96 of them Class of 2027 —
/// the counts the capture's own markup carries (`column-grad-year` appears 319 times, 96 of them
/// with the value 2027) and the census's cohort evidence.
#[test]
fn oh_roster_pins_319_graded_rows_and_96_class_of_2027() {
    let teams = parse_team_index(OH_TEAMS).unwrap();
    let mason = teams
        .iter()
        .find(|team| team.id == "10002")
        .unwrap()
        .clone();
    let roster = parse_roster(OH_ROSTER, mason).unwrap();
    // The capture carries 319 `column-grad-year` cells and the reader yields 318 of them: one cell
    // is the literal `0` (an athlete the roster publishes no graduating year for), and a row with no
    // graduating year is not a cohort row. The other 318 break down 96 x 2027, 88 x 2029, 88 x 2028,
    // 46 x 2030, which is what the assertions below pin.
    assert_eq!(roster.athletes.len(), 318);
    let co2027 = roster
        .athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027)
        .count();
    assert_eq!(co2027, 96);
    // A 2026-27 roster publishes grades 9..=12, i.e. the classes of 2027 through 2030.
    assert!(roster
        .athletes
        .iter()
        .all(|athlete| (2027..=2030).contains(&athlete.grad_year.get())));
}

/// One `/raw` body is the whole result set: 80 rows in 2 sections, with the meet identity, the
/// sport and the region read from the page's schema.org block.
#[test]
fn raw_result_set_body_pins_80_rows_in_two_sections() {
    let page = parse_raw(OH_RAW, OH_RAW_URL).unwrap();
    assert_eq!(page.meet.rows_parsed, 80);
    assert_eq!(page.meet.rows_skipped, 0);
    assert!(page.skipped.is_empty(), "skipped: {:?}", page.skipped);
    assert_eq!(page.meet.name, "Beaver Eastern Invite");
    assert_eq!(page.meet.date, "2026-09-19");
    assert_eq!(page.meet.end_date.as_deref(), Some("2026-09-19"));
    assert_eq!(page.sport, Some(Sport::CrossCountry));
    assert_eq!(page.region.as_deref(), Some("OH"));
    assert_eq!(
        page.school_year,
        SchoolYear::new(2026).expect("2026 is a season")
    );
    assert_eq!(page.meet.events.len(), 2);
    let boys = &page.meet.events[0];
    assert_eq!(boys.label, "Boys Middle School 3000 Meter");
    assert_eq!(boys.gender, Gender::Boys);
    assert_eq!(boys.kind, EventKind::CrossCountry);
    assert_eq!(boys.rows.len(), 40);
    let girls = &page.meet.events[1];
    assert_eq!(girls.label, "Girls Middle School 3000 Meter");
    assert_eq!(girls.gender, Gender::Girls);
    assert_eq!(girls.rows.len(), 40);
    let first = &boys.rows[0];
    assert_eq!(first.place, Some(1));
    assert_eq!(first.name, "Jeydyn Fields");
    assert_eq!(first.school, "Jackson");
    // The capture is a middle-school invite: all 80 rows publish a `Yr` (47 x 8, 33 x 7) and
    // `Grade` — the domain's high-school grade — admits neither, so the reader claims no grade here
    // rather than widening the domain to a grade the census does not count.
    assert_eq!(first.grade, None);
    assert!(matches!(first.mark, Mark::TimeSeconds(_)));
}

/// What this capture supports: it is a **middle-school** invite, so every `Yr` is 8 and none of them
/// is census grade evidence. The mapper counts each dropped row under its own reason and still mints
/// the meet and its events, so nothing partial is written and nothing vanishes silently.
#[test]
fn raw_rows_without_a_high_school_grade_are_counted_not_minted() {
    let page = parse_raw(OH_RAW, OH_RAW_URL).unwrap();
    let reference = ResultSetRef::parse(OH_RAW_URL).unwrap();
    let index = SchoolIndex::from_schools(&schools_of(&page));
    let mut resolved = std::collections::HashMap::new();
    let mut stats = Stats::default();
    let mut accumulated = Accumulator::default();
    let written = absorb_result_set(
        &page,
        &reference,
        "2026-09-22",
        &index,
        &mut resolved,
        &mut stats,
        &mut accumulated,
    );
    assert_eq!(written, 0);
    assert_eq!(stats.rows, 80);
    assert_eq!(stats.rows_with_grade, 0);
    assert_eq!(stats.rows_without_grade, 80);
    assert_eq!(stats.rows_school_named, 0);
    assert_eq!(stats.skipped_lines, 0);
    assert!(accumulated.athletes.is_empty());
    assert!(accumulated.performances.is_empty());
    assert!(accumulated.teams.is_empty());
    assert_eq!(accumulated.meets.len(), 1);
    assert_eq!(accumulated.events.len(), 2);
    let meet = accumulated.meets.values().next().unwrap();
    assert_eq!(meet.name, "Beaver Eastern Invite");
    assert_eq!(meet.date, "2026-09-19");
    assert_eq!(meet.state, Some(UsJurisdiction::from_code("OH").unwrap()));
    assert!(meet
        .source_identities
        .iter()
        .any(|identity| identity.id == "770621"));
}

/// The grade-evidence path itself: the same genuine body with every row's `Yr` cell — all 80 rows
/// publish one, `8` or `7` — rewritten to a high-school `10` in the same two columns the capture's
/// own header marks `Yr`. No high-school
/// `/raw` body exists in this repo — the lane kept one capture and it is middle school — so this
/// path is proven on a cell-for-cell edit of a real payload, not on a real HS payload, and the
/// unverified part is the source, not the reading: on a HS `/raw` body the same columns would hold
/// 9..=12.
#[test]
fn a_high_school_grade_is_carried_as_dated_evidence() {
    let body = with_high_school_grades(OH_RAW);
    let page = parse_raw(&body, OH_RAW_URL).unwrap();
    let reference = ResultSetRef::parse(OH_RAW_URL).unwrap();
    let index = SchoolIndex::from_schools(&schools_of(&page));
    let mut resolved = std::collections::HashMap::new();
    let mut stats = Stats::default();
    let mut accumulated = Accumulator::default();
    let written = absorb_result_set(
        &page,
        &reference,
        "2026-09-22",
        &index,
        &mut resolved,
        &mut stats,
        &mut accumulated,
    );
    assert_eq!(written, 80, "stats: {stats:?}");
    assert_eq!(stats.rows, 80);
    assert_eq!(stats.rows_with_grade, 80);
    assert_eq!(stats.rows_without_grade, 0);
    assert_eq!(accumulated.athletes.len(), 80, "stats: {stats:?}");
    assert_eq!(accumulated.performances.len(), 80);

    let athlete = accumulated
        .athletes
        .values()
        .find(|athlete| athlete.canonical_name == "Jeydyn Fields")
        .expect("the first section's winner");
    let observation = athlete.observed_grades.first().expect("grade evidence");
    assert_eq!(observation.grade.get(), 10);
    assert_eq!(
        observation.school_year,
        SchoolYear::new(2026).expect("2026 is a season")
    );
    assert_eq!(observation.source.id, "milesplit_oh");
    assert_eq!(observation.source.url.as_deref(), Some(OH_RAW_URL));
    // Grade 10 in 2026-27 graduates in 2029: the projection of the two evidence fields, not a year
    // the file published.
    assert_eq!(
        athlete.grad_year,
        GradYear::of(observation.grade, observation.school_year)
    );
    assert_eq!(athlete.grad_year, GradYear::new(2029).unwrap());

    let performance = accumulated
        .performances
        .values()
        .find(|performance| performance.athlete == athlete.id)
        .expect("the winner's performance");
    assert_eq!(performance.observed_grade.map(Grade::get), Some(10));
    assert_eq!(performance.round, None);
    let note = performance.evidence.first().unwrap().note.clone().unwrap();
    assert!(note.contains("RSID 1321880"), "note: {note}");
    assert!(note.contains("Yr 10 published on the row"), "note: {note}");
    assert!(note.contains("school year 2026"), "note: {note}");
}

/// The capture's body with every row's `Yr` cell rewritten from the middle-school `8` to a
/// high-school `10` — same two columns, same width, so the column map reads it as the capture's own
/// rows do.
fn with_high_school_grades(body: &str) -> String {
    body.lines()
        .map(|line| {
            let mut cells: Vec<char> = line.chars().collect();
            let is_graded_row = cells.get(31..33).is_some_and(|cell| {
                let text: String = cell.iter().collect();
                let trimmed = text.trim();
                trimmed.len() == 1 && trimmed.chars().all(|digit| digit.is_ascii_digit())
            });
            if is_graded_row {
                cells[31] = '1';
                cells[32] = '0';
            }
            let mut out: String = cells.into_iter().collect();
            out.push('\n');
            out
        })
        .collect()
}

/// A row that does not fit the column map is reported, not guessed: with one separator column filled
/// the line is dropped, the result set still parses, and the row count drops by exactly one.
#[test]
fn raw_row_that_does_not_fit_the_column_map_is_reported() {
    let mutated: String = OH_RAW
        .lines()
        .map(|line| match line.contains("Jeydyn Fields") {
            // Column 75 (1-based) is the separator between the team and mark fields.
            true => {
                let (head, tail) = line.split_at(74);
                format!("{head}X{tail}\n")
            }
            false => format!("{line}\n"),
        })
        .collect();
    let page = parse_raw(&mutated, OH_RAW_URL).unwrap();
    assert_eq!(page.meet.rows_parsed, 79);
    assert_eq!(page.skipped.len(), 1);
    assert!(
        page.skipped[0].contains("row did not fit the column map"),
        "skipped: {:?}",
        page.skipped
    );
    assert_eq!(page.meet.events[0].rows.len(), 39);
    assert_eq!(page.meet.rows_skipped, 1);
}

/// The URL shapes this adapter will request: a state host, `/meets/<id>/results/<rsid>/raw`. The
/// robots-disallowed `/api/`, the JS-shell `/formatted` view and the national `www` channel are not
/// routes, so an operator entry naming one is rejected rather than requested.
#[test]
fn result_set_urls_are_checked_and_disallowed_routes_are_never_built() {
    let reference = ResultSetRef::parse(OH_RAW_URL).unwrap();
    assert_eq!(reference.meet_id, "770621");
    assert_eq!(reference.rsid, "1321880");
    assert_eq!(reference.site.code(), "OH");
    assert_eq!(reference.url, OH_RAW_URL);
    for rejected in [
        // `Disallow: /api/` on every captured host, and the API lives on another host entirely.
        "https://oh.milesplit.com/api/v1/meets/770621/performances",
        "https://api.prod.milesplit.com/v1/meets/770621/performances",
        // The formatted sibling renders 0 rows: a JS shell over the API.
        "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/formatted",
        // The national channel is not a jurisdiction.
        "https://www.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/raw",
        // Not a result set at all.
        "https://oh.milesplit.com/teams",
    ] {
        assert!(ResultSetRef::parse(rejected).is_none(), "accepted {rejected}");
    }
    for jurisdiction in UsJurisdiction::ALL {
        let site = Site::for_jurisdiction(jurisdiction);
        let url = site.teams_url();
        assert!(
            url.starts_with(&format!(
                "https://{}.milesplit.com/teams",
                jurisdiction.code().to_ascii_lowercase()
            )) && !url.contains("/api/"),
            "url: {url}"
        );
    }
}

/// The consolidated schools a result-set run resolves against, derived from the capture's own
/// labels so the mapping test exercises the resolver end to end.
fn schools_of(page: &RawPage) -> Vec<CanonicalSchool> {
    let mut labels: Vec<&str> = page
        .meet
        .events
        .iter()
        .flat_map(|event| event.rows.iter())
        .map(|row| row.school.as_str())
        .filter(|label| !label.trim().is_empty())
        .collect();
    labels.sort_unstable();
    labels.dedup();
    labels
        .into_iter()
        .map(|label| {
            CanonicalSchool::new(
                UsJurisdiction::from_code("OH").unwrap(),
                label,
                normalize_name(label),
            )
            .0
        })
        .collect()
}

#[test]
fn parses_a_state_results_index_into_requestable_meets() {
    // The census's meet enumeration: one page of a state results index, captured 2026-09-22.
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
    // The row publishes `Sep 19` and its section publishes `2026-09`; the date is both.
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
    // The reader's contract is a requestable meet: a row it cannot address is dropped rather than
    // published with an empty id, and a row whose day is not a day of its month keeps no date.
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

// ---------------------------------------------------------------------------
// The results page's own file list. Fixture:
// `tests/fixtures/milesplit/oh_meet_770621_results.html`, the same capture the fetch log records
// (`samples/fetch-log-wave2.tsv`: 200, 45,308 B, 2026-09-22T03:55:44Z), copied byte-for-byte.
// ---------------------------------------------------------------------------

/// The page lists its result files, and the id it lists is the id the `/raw` route serves: the
/// capture's own file list (`meetResultFiles = [{"id":1321880,"name":"Results","isMeetPro":0}]`)
/// names the same `RSID` the result-set sample was fetched under.
#[test]
fn a_results_page_lists_the_result_files_the_raw_route_serves() {
    let files = parse_meet_result_files(OH_MEET_RESULTS_URL, OH_MEET_RESULTS).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].id, 1321880);
    assert_eq!(files[0].name, "Results");
    assert_eq!(files[0].is_meet_pro, 0);
}

/// A listed file's URL is the URL the site serves, and it checks back into the same meet and
/// result set: discovery and the `/raw` reader agree on the address without a second convention.
#[test]
fn a_listed_result_file_addresses_the_raw_url_the_reader_accepts() {
    let files = parse_meet_result_files(OH_MEET_RESULTS_URL, OH_MEET_RESULTS).unwrap();
    let url = files[0].raw_url(OH_MEET_RESULTS_URL);
    assert_eq!(url, OH_RAW_URL);
    let reference = ResultSetRef::parse(&url).expect("the derived URL is a /raw URL");
    assert_eq!(reference.meet_id, "770621");
    assert_eq!(reference.rsid, "1321880");
}

/// A page that publishes no file list at all is a schema mismatch, not an empty meet: the list is
/// the only cheap record of what result sets exist, so its absence is reported.
#[test]
fn a_page_without_a_file_list_is_a_schema_mismatch() {
    let error = parse_meet_result_files(
        "https://www.milesplit.com/meets/1-a/results",
        "<html><body>no result files here</body></html>",
    )
    .expect_err("no file list is a mismatch");
    assert!(matches!(error, CrawlError::Schema { .. }), "{error:?}");
}

/// A meet with no results yet publishes an empty list, which is a state rather than a failure.
#[test]
fn an_empty_file_list_is_a_state_not_a_failure() {
    let files = parse_meet_result_files(OH_MEET_RESULTS_URL, "let meetResultFiles = [];").unwrap();
    assert!(files.is_empty());
}

// ---------------------------------------------------------------------------
// Defect 1: legacy <select id="ddResultsPage"> template.
// ---------------------------------------------------------------------------

/// The DC fixture uses the legacy `<select id="ddResultsPage">` template instead of the
/// `meetResultFiles` JS literal. This test proves the parser reads it into the same shape.
#[test]
fn dc_legacy_fixture_parses_result_file_entries() {
    const DC_LEGACY: &str =
        include_str!("../../tests/fixtures/milesplit/dc_meet_735841_results_legacy.html");
    let files = parse_meet_result_files(
        "https://www.milesplit.com/meets/735841-stancs-home-meet-1-2026/results",
        DC_LEGACY,
    )
    .unwrap();
    // The fixture carries two per-file options (Varsity Boys + Varsity Girls), not the All option.
    assert_eq!(files.len(), 2, "two per-file result entries");
    assert_eq!(files[0].id, 1257095);
    assert_eq!(files[0].name, "Varsity Boys Results");
    assert_eq!(files[0].is_meet_pro, 0);
    assert_eq!(files[1].id, 1257096);
    assert_eq!(files[1].name, "Varsity Girls Results");
    assert_eq!(files[1].is_meet_pro, 0);
}

/// A legacy select that contains only the `All` option is the legacy spelling of the empty list.
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

// ---------------------------------------------------------------------------
// Defect 3: the inline results page — no file list, the page is the result set.
// ---------------------------------------------------------------------------

/// The DC10 Track Fest capture (`meet 764735`) is the third template: no file list at all, and the
/// meet's one result set is the `<pre>` block on the results page itself. It is read as one inline
/// file whose address is the page, and the page parses as a `/raw` body — which is what lets the
/// route carry these rows without a `/raw` of their own.
#[test]
fn an_inline_results_page_is_its_own_one_result_set() {
    const DC_INLINE: &str =
        include_str!("../../tests/fixtures/milesplit/dc_meet_764735_results_inline.html");
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

// ---------------------------------------------------------------------------
// Defect 2: www.milesplit.com host handling.
// ---------------------------------------------------------------------------

/// A www-hosted URL is rejected by `parse` (the original path) but accepted by
/// `parse_with_jurisdiction` with the caller-supplied jurisdiction.
#[test]
fn www_host_is_rejected_by_parse_but_accepted_by_parse_with_jurisdiction() {
    let www_url =
        "https://www.milesplit.com/meets/735841-stancs-home-meet-1-2026/results/1257095/raw";
    // Original parse: www is not a jurisdiction code → rejected.
    assert!(
        ResultSetRef::parse(www_url).is_none(),
        "www host rejected by plain parse"
    );
    // With jurisdiction: accepted, attributed to the caller's jurisdiction.
    let ref_with_jur =
        ResultSetRef::parse_with_jurisdiction(www_url, UsJurisdiction::DistrictOfColumbia);
    let reference = ref_with_jur.expect("www host accepted with explicit jurisdiction");
    assert_eq!(reference.meet_id, "735841");
    assert_eq!(reference.rsid, "1257095");
    assert_eq!(reference.site.code(), "DC");
    assert_eq!(reference.url, www_url);
}

/// An unknown host (neither state nor www) is rejected by both parse methods.
#[test]
fn unknown_host_is_rejected_by_both_parse_methods() {
    let fake_url = "https://fake.milesplit.com/meets/123456-x/results/789/raw";
    assert!(ResultSetRef::parse(fake_url).is_none());
    assert!(ResultSetRef::parse_with_jurisdiction(fake_url, UsJurisdiction::Ohio).is_none());
}

/// A page with neither the JS literal nor the legacy select fails closed with a schema error.
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
