use super::*;
use census_domain::model::{normalize_name, CanonicalSchool, Grade};

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

#[test]
fn raw_row_that_does_not_fit_the_column_map_is_reported() {
    let mutated: String = OH_RAW
        .lines()
        .map(|line| match line.contains("Jeydyn Fields") {
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

#[test]
fn result_set_urls_are_checked_and_disallowed_routes_are_never_built() {
    let reference = ResultSetRef::parse(OH_RAW_URL).unwrap();
    assert_eq!(reference.meet_id, "770621");
    assert_eq!(reference.rsid, "1321880");
    assert_eq!(reference.site.code(), "OH");
    assert_eq!(reference.url, OH_RAW_URL);
    for rejected in [
        "https://oh.milesplit.com/api/v1/meets/770621/performances",
        "https://api.prod.milesplit.com/v1/meets/770621/performances",
        "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/formatted",
        "https://www.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/raw",
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
fn a_results_page_lists_the_result_files_the_raw_route_serves() {
    let files = parse_meet_result_files(OH_MEET_RESULTS_URL, OH_MEET_RESULTS).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].id, 1321880);
    assert_eq!(files[0].name, "Results");
    assert_eq!(files[0].is_meet_pro, 0);
}

#[test]
fn a_listed_result_file_addresses_the_raw_url_the_reader_accepts() {
    let files = parse_meet_result_files(OH_MEET_RESULTS_URL, OH_MEET_RESULTS).unwrap();
    let url = files[0].raw_url(OH_MEET_RESULTS_URL);
    assert_eq!(url, OH_RAW_URL);
    let reference = ResultSetRef::parse(&url).expect("the derived URL is a /raw URL");
    assert_eq!(reference.meet_id, "770621");
    assert_eq!(reference.rsid, "1321880");
}

#[test]
fn a_page_without_a_file_list_is_a_schema_mismatch() {
    let error = parse_meet_result_files(
        "https://www.milesplit.com/meets/1-a/results",
        "<html><body>no result files here</body></html>",
    )
    .expect_err("no file list is a mismatch");
    assert!(matches!(error, CrawlError::Schema { .. }), "{error:?}");
}

#[test]
fn an_empty_file_list_is_a_state_not_a_failure() {
    let files = parse_meet_result_files(OH_MEET_RESULTS_URL, "let meetResultFiles = [];").unwrap();
    assert!(files.is_empty());
}
