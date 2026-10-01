use crate::CrawlError;
use census_domain::model::{EventKind, Gender, Mark};

use super::super::raw::parse_raw;
use super::super::{RawGradeIssueKind, SourceRowLocator};

const URL: &str = "https://www.milesplit.com/meets/498412/results/1283641/raw";

fn document(block: &str) -> String {
    format!(
        "<script type=\"application/ld+json\">{{\"name\":\"Camp\",\"startDate\":\"2025-03-08\",\"sport\":\"Track\"}}</script><pre>{block}</pre>"
    )
}

fn page(block: &str) -> super::super::raw::RawPage {
    parse_raw(&document(block), URL).expect("qualified raw document")
}

fn header(name_width: usize, team_width: usize) -> String {
    format!(
        "  Pl {:<name_width$} {:<3} {:<team_width$}{:>9}",
        "Name", "Yr", "Team", "Time"
    )
}

fn row(
    name_width: usize,
    team_width: usize,
    name: &str,
    grade: &str,
    school: &str,
    mark: &str,
) -> String {
    format!(
        "   1 {:<name_width$} {:<3} {:<team_width$}{:>9} 8 (1)",
        name, grade, school, mark
    )
}

fn assert_locator(locator: &SourceRowLocator, block: &str, ordinal: u32, offset: usize, row: &str) {
    let html = document(block);
    let document_offset = html.find(row).expect("original published row");
    assert_eq!(block.find(row), Some(offset));
    assert_eq!(locator.ordinal, ordinal);
    assert_eq!(locator.byte_offset, document_offset);
    assert_eq!(locator.byte_length, row.len());
    let end = document_offset
        .checked_add(locator.byte_length)
        .expect("row end");
    assert_eq!(html.get(document_offset..end), Some(row));
}

#[test]
fn published_metric_field_row_keeps_event_grade_and_numeric_distance() {
    let result = row(20, 20, "Captured, Jumper", "11", "Captured School", "9.47m");
    let block = format!(
        "Girls Varsity Triple Jump Finals\n{}\n{result}\n",
        header(20, 20).replace("Time", "Mark")
    );
    let parsed = page(&block);
    let event = parsed.meet.events.first().expect("published event");
    assert_eq!(event.kind, EventKind::TripleJump);
    let result = event.rows.first().expect("published result");
    assert_eq!(
        result.mark,
        Mark::DistanceMetres(census_domain::model::CentiMetres::new(947))
    );
    assert_eq!(result.grade.map(|grade| grade.get()), Some(11));
    assert_eq!(result.timing, None);
}

#[test]
fn a_wrapped_round_does_not_relabel_the_earlier_results() {
    let first = row(20, 20, "MÜLLER, Émile", "JR", "North Buncombe", "9:01.03");
    let second = row(20, 20, "Other, Kid", "SR", "Asheville", "9:02.04");
    let block = format!(
        "Boys 3200M\n{}\n{first}\n======\nFinals\n{}\n{second}\n",
        header(20, 20),
        header(20, 20)
    );
    let parsed = page(&block);
    assert_eq!(parsed.meet.events.len(), 2);
    assert_eq!(parsed.meet.events[0].label, "Boys 3200M");
    assert_eq!(parsed.meet.events[1].label, "Boys 3200M Finals");
    assert_eq!(parsed.meet.events[1].round.as_deref(), Some("Finals"));
    assert_eq!(parsed.meet.events[0].rows[0].name, "MÜLLER, Émile");
    assert_eq!(
        parsed.meet.events[0].rows[0].grade.map(|grade| grade.get()),
        Some(11)
    );
    assert_eq!(parsed.meet.events[1].rows[0].name, "Other, Kid");
    assert_eq!(
        parsed.meet.events[1].rows[0].grade.map(|grade| grade.get()),
        Some(12)
    );
    assert_eq!(
        parsed.meet.events[0].rows[0].mark,
        Mark::TimeSeconds(crate::hytek::parse_time("9:01.03").unwrap())
    );
    assert_eq!(parsed.skipped, Vec::<String>::new());
    assert_eq!(parsed.grade_issues, Vec::new());
    assert_eq!(parsed.meet.rows_parsed, 2);
    assert_eq!(parsed.meet.rows_skipped, 0);
}

#[test]
fn a_header_derived_mark_column_does_not_hide_a_named_row() {
    let name = "Cadet, Morgan Alexander";
    let line = row(24, 20, name, "JR", "Mountain View", "11.02");
    let block = format!("Boys 100M\n{}\n{line}\n", header(24, 20));
    let parsed = page(&block);
    assert_eq!(parsed.skipped, Vec::<String>::new());
    assert_eq!(parsed.grade_issues, Vec::new());
    assert_eq!(parsed.meet.rows_parsed, 1);
    assert_eq!(parsed.meet.rows_skipped, 0);
    let result = &parsed.meet.events[0].rows[0];
    assert_eq!(result.name, name);
    assert_eq!(result.school, "Mountain View");
    assert_eq!(result.grade.map(|grade| grade.get()), Some(11));
    assert_eq!(
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time("11.02").unwrap())
    );
    assert_eq!(result.heat.as_deref(), Some("8"));
}

#[test]
fn a_located_grade_eight_row_is_preserved_not_skipped() {
    const SCHOOL: &str = "North Buncombe";
    let line = row(20, 20, "SURFACE, Luke", "8", SCHOOL, "10:31.14");
    let block = format!("Boys 3200M\n{}\n{line}\n", header(20, 20));
    let parsed = page(&block);
    assert_eq!(parsed.skipped, Vec::<String>::new());
    assert_eq!(parsed.meet.rows_parsed, 1);
    assert_eq!(parsed.meet.rows_skipped, 0);
    let issue = parsed
        .grade_issues
        .first()
        .expect("the eighth-grade token is located");
    assert_eq!(issue.kind, RawGradeIssueKind::OutsideHighSchool);
    assert_eq!(issue.raw_token, "8");
    assert_locator(&issue.row, &block, 3, block.find(&line).unwrap(), &line);
    let result = &parsed.meet.events[0].rows[0];
    assert_eq!(result.name, "SURFACE, Luke");
    assert_eq!(result.school, SCHOOL);
    assert_eq!(result.grade, None);
    assert_eq!(
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time("10:31.14").unwrap())
    );
    assert_eq!(result.heat.as_deref(), Some("8"));
}

#[test]
fn a_college_grade_token_is_a_located_field_exclusion() {
    let line = row(20, 20, "SURFACE, Luke", "13", "North Buncombe", "10:31.14");
    let block = format!("Boys 3200M\n{}\n{line}\n", header(20, 20));
    let parsed = page(&block);
    assert_eq!(parsed.skipped, Vec::<String>::new());
    assert_eq!(parsed.meet.rows_parsed, 1);
    assert_eq!(parsed.meet.rows_skipped, 0);
    let issue = parsed
        .grade_issues
        .first()
        .expect("the college-grade token is located");
    assert_eq!(issue.kind, RawGradeIssueKind::OutsideHighSchool);
    assert_eq!(issue.raw_token, "13");
    assert_eq!(issue.row.ordinal, 3);
    let result = &parsed.meet.events[0].rows[0];
    assert_eq!(result.name, "SURFACE, Luke");
    assert_eq!(result.grade, None);
    assert_eq!(
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time("10:31.14").unwrap())
    );
}

#[test]
fn a_missing_grade_is_not_a_located_issue() {
    let line = row(20, 20, "SURFACE, Luke", "-", "North Buncombe", "10:31.14");
    let block = format!("Boys 3200M\n{}\n{line}\n", header(20, 20));
    let parsed = page(&block);
    assert_eq!(parsed.skipped, Vec::<String>::new());
    assert_eq!(parsed.grade_issues, Vec::new());
    assert_eq!(parsed.meet.rows_parsed, 1);
    assert_eq!(parsed.meet.rows_skipped, 0);
    let result = &parsed.meet.events[0].rows[0];
    assert_eq!(result.grade, None);
    assert_eq!(
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time("10:31.14").unwrap())
    );
}

#[test]
fn a_malformed_grade_token_owes_a_partial_result_set() {
    let line = row(20, 20, "SURFACE, Luke", "--", "North Buncombe", "9:00.11");
    let block = format!("Boys 3200M\n{}\n{line}\n", header(20, 20));
    let parsed = page(&block);
    assert_eq!(parsed.skipped, Vec::<String>::new());
    assert_eq!(parsed.meet.rows_parsed, 1);
    assert_eq!(parsed.meet.rows_skipped, 0);
    assert_eq!(parsed.grade_issues.len(), 1);
    let issue = &parsed.grade_issues[0];
    assert_eq!(issue.kind, RawGradeIssueKind::Unrecognized);
    assert_eq!(issue.raw_token, "--");
    assert_locator(&issue.row, &block, 3, block.find(&line).unwrap(), &line);
    let result = &parsed.meet.events[0].rows[0];
    assert_eq!(result.name, "SURFACE, Luke");
    assert_eq!(result.school, "North Buncombe");
    assert_eq!(result.grade, None);
    assert_eq!(
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time("9:00.11").unwrap())
    );
}

#[test]
fn numeric_school_labels_keep_the_published_mark() {
    const SCHOOL: &str = "District 5-12 Academy";
    let located = row(20, 24, "MÜLLER, Émile", "JR", SCHOOL, "8:44.73");
    let parsed = page(&format!("Boys 3200M\n{}\n{located}\n", header(20, 24)));
    assert_eq!(parsed.skipped, Vec::<String>::new());
    assert_eq!(parsed.grade_issues, Vec::new());
    assert_eq!(parsed.meet.rows_parsed, 1);
    assert_eq!(parsed.meet.rows_skipped, 0);
    let result = &parsed.meet.events[0].rows[0];
    assert_eq!(result.name, "MÜLLER, Émile");
    assert_eq!(result.school, SCHOOL);
    assert_eq!(result.grade.map(|grade| grade.get()), Some(11));
    assert_eq!(
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time("8:44.73").unwrap())
    );
    assert_eq!(result.heat.as_deref(), Some("8"));
}

#[test]
fn located_byte_offsets_resolve_the_original_document() {
    let line = row(20, 20, "SURFACE, Luke", "8", "North Buncombe", "10:31.14");
    for ending in ["\n", "\r\n"] {
        let html = format!(
            "Captured π document<script type=\"application/ld+json\">{{\"name\":\"Camp\",\"startDate\":\"2025-03-08\",\"sport\":\"Track\"}}</script><pre>Boys 3200M{ending}{}{ending}{line}{ending}</pre>",
            header(20, 20)
        );
        let parsed = parse_raw(&html, URL).expect("qualified raw document");
        let issue = parsed
            .grade_issues
            .first()
            .expect("the grade issue is reported");
        let start = html.find(&line).expect("the raw row sits in the document");
        assert_eq!(issue.row.byte_offset, start);
        assert_eq!(
            html.get(
                issue.row.byte_offset
                    ..issue
                        .row
                        .byte_offset
                        .checked_add(issue.row.byte_length)
                        .expect("located range")
            ),
            Some(line.as_str()),
            "line ending {ending:?}"
        );
    }
}

#[test]
fn a_row_without_a_column_map_is_rejected_with_its_bytes() {
    let html = "<script type=\"application/ld+json\">{\"name\":\"Camp\",\"startDate\":\"2025-03-08\",\"sport\":\"Track\"}</script><pre>Boys 3200M\n1 No Header, Kid SR North Buncombe 9:01.03 8</pre>";
    let error = parse_raw(html, URL).expect_err("a row without a column map is rejected");
    let CrawlError::Schema { detail, .. } = error else {
        panic!("expected a schema rejection, got {error:?}");
    };
    assert!(detail.contains("no qualified column header"), "{detail}");
    assert!(detail.contains("No Header, Kid"), "{detail}");
}

#[test]
fn an_unqualified_document_is_not_a_successful_empty_result_set() {
    let html = "<script type=\"application/ld+json\">{\"name\":\"Camp\",\"startDate\":\"2025-03-08\"}</script><pre>Boys 3200M\nName Team\n   1 MÜLLER, Émile</pre>";
    match parse_raw(html, URL) {
        Err(CrawlError::Schema { url, detail }) => {
            assert_eq!(url, URL);
            assert!(detail.contains("unqualified raw result document"));
            assert!(detail.contains("no qualified column header"));
        }
        other => panic!("expected an unqualified rejection, got {other:?}"),
    }
}

#[test]
fn a_mark_wider_than_the_published_cell_keeps_its_school() {
    const MARK: &str = "1:02:03.45";
    let line = format!(
        "   1 {:<20} {:<3} {:<19}{:>10} 8 (1)",
        "SURFACE, Luke", "JR", "District 5-12", MARK
    );
    let block = format!("Boys 3200M\n{}\n{line}\n", header(20, 20));
    let parsed = page(&block);
    assert_eq!(parsed.skipped, Vec::<String>::new());
    assert_eq!(parsed.grade_issues, Vec::new());
    assert_eq!(parsed.meet.rows_parsed, 1);
    assert_eq!(parsed.meet.rows_skipped, 0);
    let result = &parsed.meet.events[0].rows[0];
    assert_eq!(result.place, Some(1));
    assert_eq!(result.name, "SURFACE, Luke");
    assert_eq!(result.school, "District 5-12");
    assert_eq!(result.grade.map(|grade| grade.get()), Some(11));
    assert_eq!(
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time(MARK).unwrap())
    );
    assert_eq!(result.heat.as_deref(), Some("8"));
}

#[test]
fn a_blank_declared_mark_cell_never_promotes_school_digits() {
    const SCHOOL: &str = "District 5-12 Academy";
    const SPILLED: &str = "District 5-12 Academy 2024";
    let blank = format!(
        "   1 {:<20} {:<3} {:<24}{:>9} 8 (1)",
        "SURFACE, Luke", "JR", SCHOOL, ""
    );
    let blank_block = format!("Boys 3200M\n{}\n{blank}\n", header(20, 24));
    let parsed = page(&blank_block);
    assert_eq!(parsed.meet.events[0].rows.len(), 0);
    assert_eq!(parsed.meet.rows_parsed, 0);
    assert_eq!(parsed.meet.rows_skipped, 1);
    assert_eq!(parsed.skipped.len(), 1);
    assert!(
        parsed.skipped[0].contains("missing or malformed mark"),
        "skipped: {:?}",
        parsed.skipped
    );
    let spilled = format!(
        "   1 {:<20} {:<3} {:<24}{:>9} 8 (1)",
        "SURFACE, Luke", "JR", SPILLED, ""
    );
    let spilled_block = format!("Boys 3200M\n{}\n{spilled}\n", header(20, 24));
    let spilled_page = page(&spilled_block);
    assert_eq!(spilled_page.meet.events[0].rows.len(), 0);
    assert_eq!(spilled_page.meet.rows_parsed, 0);
    assert_eq!(spilled_page.skipped.len(), 1);
    assert!(
        spilled_page.skipped[0].contains("row did not fit the column map"),
        "skipped: {:?}",
        spilled_page.skipped
    );
}

#[test]
fn escaped_section_labels_decode_before_publication() {
    let line = row(20, 20, "SURFACE, Luke", "JR", "North Buncombe", "44.73");
    let block = format!("Girls&#039; Javelin\n{}\n{line}\n", header(20, 20));
    let parsed = page(&block);
    assert_eq!(parsed.skipped, Vec::<String>::new());
    assert_eq!(parsed.grade_issues, Vec::new());
    assert_eq!(parsed.meet.rows_parsed, 1);
    assert_eq!(parsed.meet.rows_skipped, 0);
    let event = &parsed.meet.events[0];
    assert_eq!(event.label, "Girls' Javelin");
    assert_eq!(event.gender, Gender::Girls);
    assert_eq!(event.kind, EventKind::Javelin);
    assert_eq!(event.rows.len(), 1);
    assert_eq!(event.rows[0].school, "North Buncombe");
    assert!(matches!(event.rows[0].mark, Mark::DistanceMetres(_)));
    assert_eq!(event.rows[0].heat.as_deref(), Some("8"));
}
