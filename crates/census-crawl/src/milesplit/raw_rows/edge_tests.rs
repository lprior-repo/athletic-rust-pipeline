use crate::CrawlError;
use census_domain::model::{EventKind, Gender, Mark};

use super::super::raw::parse_raw;
use super::super::{RawGradeIssue, RawGradeIssueKind, SourceRowLocator};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const URL: &str = "https://www.milesplit.com/meets/498412/results/1283641/raw";

fn document(block: &str) -> String {
    format!(
        "<script type=\"application/ld+json\">{{\"name\":\"Camp\",\"startDate\":\"2025-03-08\",\"sport\":\"Track\"}}</script><pre>{block}</pre>"
    )
}

fn page(block: &str) -> TestResult<super::super::raw::RawPage> {
    Ok(parse_raw(&document(block), URL)?)
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

fn assert_locator(
    locator: &SourceRowLocator,
    block: &str,
    ordinal: u32,
    offset: usize,
    row: &str,
) -> TestResult {
    let html = document(block);
    let document_offset = html.find(row).ok_or("original published row")?;
    check!(eq; block.find(row), Some(offset));
    check!(eq; locator.ordinal, ordinal);
    check!(eq; locator.byte_offset, document_offset);
    check!(eq; locator.byte_length, row.len());
    let end = document_offset
        .checked_add(locator.byte_length)
        .ok_or("row end")?;
    check!(eq; html.get(document_offset..end), Some(row));
    Ok(())
}

#[test]
fn published_metric_field_row_keeps_event_grade_and_numeric_distance() -> TestResult {
    let result = row(20, 20, "Captured, Jumper", "11", "Captured School", "9.47m");
    let block = format!(
        "Girls Varsity Triple Jump Finals\n{}\n{result}\n",
        header(20, 20).replace("Time", "Mark")
    );
    let parsed = page(&block)?;
    let event = parsed.meet.events.first().ok_or("published event")?;
    check!(eq; event.kind, EventKind::TripleJump);
    let result = event.rows.first().ok_or("published result")?;
    check!(eq;
        result.mark,
        Mark::DistanceMetres(census_domain::model::CentiMetres::new(947))
    );
    check!(eq; result.grade.map(|grade| grade.get()), Some(11));
    check!(eq; result.timing, None);
    Ok(())
}

#[test]
fn a_wrapped_round_does_not_relabel_the_earlier_results() -> TestResult {
    let first = row(20, 20, "MÜLLER, Émile", "JR", "North Buncombe", "9:01.03");
    let second = row(20, 20, "Other, Kid", "SR", "Asheville", "9:02.04");
    let block = format!(
        "Boys 3200M\n{}\n{first}\n======\nFinals\n{}\n{second}\n",
        header(20, 20),
        header(20, 20)
    );
    let parsed = page(&block)?;
    check!(eq; parsed.meet.events.len(), 2);
    check!(eq; parsed.meet.events[0].label, "Boys 3200M");
    check!(eq; parsed.meet.events[1].label, "Boys 3200M Finals");
    check!(eq; parsed.meet.events[1].round.as_deref(), Some("Finals"));
    check!(eq; parsed.meet.events[0].rows[0].name, "MÜLLER, Émile");
    check!(eq;
        parsed.meet.events[0].rows[0].grade.map(|grade| grade.get()),
        Some(11)
    );
    check!(eq; parsed.meet.events[1].rows[0].name, "Other, Kid");
    check!(eq;
        parsed.meet.events[1].rows[0].grade.map(|grade| grade.get()),
        Some(12)
    );
    check!(eq;
        parsed.meet.events[0].rows[0].mark,
        Mark::TimeSeconds(crate::hytek::parse_time("9:01.03").ok_or("published mark")?)
    );
    check!(eq; parsed.skipped, Vec::<String>::new());
    check!(eq; parsed.grade_issues, Vec::new());
    check!(eq; parsed.meet.rows_parsed, 2);
    check!(eq; parsed.meet.rows_skipped, 0);
    Ok(())
}

#[test]
fn a_header_derived_mark_column_does_not_hide_a_named_row() -> TestResult {
    let name = "Cadet, Morgan Alexander";
    let line = row(24, 20, name, "JR", "Mountain View", "11.02");
    let block = format!("Boys 100M\n{}\n{line}\n", header(24, 20));
    let parsed = page(&block)?;
    check!(eq; parsed.skipped, Vec::<String>::new());
    check!(eq; parsed.grade_issues, Vec::new());
    check!(eq; parsed.meet.rows_parsed, 1);
    check!(eq; parsed.meet.rows_skipped, 0);
    let result = &parsed.meet.events[0].rows[0];
    check!(eq; result.name, name);
    check!(eq; result.school, "Mountain View");
    check!(eq; result.grade.map(|grade| grade.get()), Some(11));
    check!(eq;
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time("11.02").ok_or("published mark")?)
    );
    check!(eq; result.heat.as_deref(), Some("8"));
    Ok(())
}

#[test]
fn a_located_grade_eight_row_is_preserved_not_skipped() -> TestResult {
    const SCHOOL: &str = "North Buncombe";
    let line = row(20, 20, "SURFACE, Luke", "8", SCHOOL, "10:31.14");
    let block = format!("Boys 3200M\n{}\n{line}\n", header(20, 20));
    let parsed = page(&block)?;
    check!(eq; parsed.skipped, Vec::<String>::new());
    check!(eq; parsed.meet.rows_parsed, 1);
    check!(eq; parsed.meet.rows_skipped, 0);
    let issue = parsed
        .grade_issues
        .first()
        .ok_or("the eighth-grade token is located")?;
    check!(eq; issue.kind, RawGradeIssueKind::OutsideHighSchool);
    check!(eq; issue.raw_token, "8");
    assert_locator(
        &issue.row,
        &block,
        3,
        block.find(&line).ok_or("published row")?,
        &line,
    )?;
    let result = &parsed.meet.events[0].rows[0];
    check!(eq; result.name, "SURFACE, Luke");
    check!(eq; result.school, SCHOOL);
    check!(eq; result.grade, None);
    check!(eq;
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time("10:31.14").ok_or("published mark")?)
    );
    check!(eq; result.heat.as_deref(), Some("8"));
    Ok(())
}

#[test]
fn a_college_grade_token_is_a_located_field_exclusion() -> TestResult {
    let line = row(20, 20, "SURFACE, Luke", "13", "North Buncombe", "10:31.14");
    let block = format!("Boys 3200M\n{}\n{line}\n", header(20, 20));
    let parsed = page(&block)?;
    check!(eq; parsed.skipped, Vec::<String>::new());
    check!(eq; parsed.meet.rows_parsed, 1);
    check!(eq; parsed.meet.rows_skipped, 0);
    let issue = parsed
        .grade_issues
        .first()
        .ok_or("the college-grade token is located")?;
    check!(eq; issue.kind, RawGradeIssueKind::OutsideHighSchool);
    check!(eq; issue.raw_token, "13");
    check!(eq; issue.row.ordinal, 3);
    let result = &parsed.meet.events[0].rows[0];
    check!(eq; result.name, "SURFACE, Luke");
    check!(eq; result.grade, None);
    check!(eq;
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time("10:31.14").ok_or("published mark")?)
    );
    Ok(())
}

#[test]
fn a_missing_grade_is_not_a_located_issue() -> TestResult {
    let line = row(20, 20, "SURFACE, Luke", "-", "North Buncombe", "10:31.14");
    let block = format!("Boys 3200M\n{}\n{line}\n", header(20, 20));
    let parsed = page(&block)?;
    check!(eq; parsed.skipped, Vec::<String>::new());
    check!(eq; parsed.grade_issues, Vec::new());
    check!(eq; parsed.meet.rows_parsed, 1);
    check!(eq; parsed.meet.rows_skipped, 0);
    let result = &parsed.meet.events[0].rows[0];
    check!(eq; result.grade, None);
    check!(eq;
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time("10:31.14").ok_or("published mark")?)
    );
    Ok(())
}

#[test]
fn a_malformed_grade_token_owes_a_partial_result_set() -> TestResult {
    let line = row(20, 20, "SURFACE, Luke", "--", "North Buncombe", "9:00.11");
    let block = format!("Boys 3200M\n{}\n{line}\n", header(20, 20));
    let parsed = page(&block)?;
    check!(eq; parsed.skipped, Vec::<String>::new());
    check!(eq; parsed.meet.rows_parsed, 1);
    check!(eq; parsed.meet.rows_skipped, 0);
    check!(eq; parsed.grade_issues.len(), 1);
    let issue = &parsed.grade_issues[0];
    check!(eq; issue.kind, RawGradeIssueKind::Unrecognized);
    check!(eq; issue.raw_token, "--");
    assert_locator(
        &issue.row,
        &block,
        3,
        block.find(&line).ok_or("published row")?,
        &line,
    )?;
    let result = &parsed.meet.events[0].rows[0];
    check!(eq; result.name, "SURFACE, Luke");
    check!(eq; result.school, "North Buncombe");
    check!(eq; result.grade, None);
    check!(eq;
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time("9:00.11").ok_or("published mark")?)
    );
    Ok(())
}

#[test]
fn numeric_school_labels_keep_the_published_mark() -> TestResult {
    const SCHOOL: &str = "District 5-12 Academy";
    let located = row(20, 24, "MÜLLER, Émile", "JR", SCHOOL, "8:44.73");
    let parsed = page(&format!("Boys 3200M\n{}\n{located}\n", header(20, 24)))?;
    check!(eq; parsed.skipped, Vec::<String>::new());
    check!(eq; parsed.grade_issues, Vec::new());
    check!(eq; parsed.meet.rows_parsed, 1);
    check!(eq; parsed.meet.rows_skipped, 0);
    let result = &parsed.meet.events[0].rows[0];
    check!(eq; result.name, "MÜLLER, Émile");
    check!(eq; result.school, SCHOOL);
    check!(eq; result.grade.map(|grade| grade.get()), Some(11));
    check!(eq;
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time("8:44.73").ok_or("published mark")?)
    );
    check!(eq; result.heat.as_deref(), Some("8"));
    Ok(())
}

#[test]
fn located_byte_offsets_resolve_the_original_document() -> TestResult {
    let line = row(20, 20, "SURFACE, Luke", "8", "North Buncombe", "10:31.14");
    for ending in ["\n", "\r\n"] {
        let html = format!(
            "Captured π document<script type=\"application/ld+json\">{{\"name\":\"Camp\",\"startDate\":\"2025-03-08\",\"sport\":\"Track\"}}</script><pre>Boys 3200M{ending}{}{ending}{line}{ending}</pre>",
            header(20, 20)
        );
        let parsed = parse_raw(&html, URL)?;
        let issue = parsed
            .grade_issues
            .first()
            .ok_or("the grade issue is reported")?;
        let start = html.find(&line).ok_or("the raw row sits in the document")?;
        check!(eq; issue.row.byte_offset, start);
        check!(eq;
            html.get(
                issue.row.byte_offset
                    ..issue
                        .row
                        .byte_offset
                        .checked_add(issue.row.byte_length)
                        .ok_or("located range")?
            ),
            Some(line.as_str()),
            "line ending {ending:?}"
        );
    }
    Ok(())
}

#[test]
fn a_row_without_a_column_map_is_rejected_with_its_bytes() -> TestResult {
    let html = "<script type=\"application/ld+json\">{\"name\":\"Camp\",\"startDate\":\"2025-03-08\",\"sport\":\"Track\"}</script><pre>Boys 3200M\n1 No Header, Kid SR North Buncombe 9:01.03 8</pre>";
    let error = match parse_raw(html, URL) {
        Err(error) => error,
        Ok(_) => return Err("a row without a column map is rejected".into()),
    };
    let CrawlError::Schema { detail, .. } = error else {
        return Err(format!("expected a schema rejection, got {error:?}").into());
    };
    check!(detail.contains("no qualified column header"), "{detail}");
    check!(detail.contains("No Header, Kid"), "{detail}");
    Ok(())
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
fn a_mark_wider_than_the_published_cell_keeps_its_school() -> TestResult {
    const MARK: &str = "1:02:03.45";
    let line = format!(
        "   1 {:<20} {:<3} {:<19}{:>10} 8 (1)",
        "SURFACE, Luke", "JR", "District 5-12", MARK
    );
    let block = format!("Boys 3200M\n{}\n{line}\n", header(20, 20));
    let parsed = page(&block)?;
    check!(eq; parsed.skipped, Vec::<String>::new());
    check!(eq; parsed.grade_issues, Vec::new());
    check!(eq; parsed.meet.rows_parsed, 1);
    check!(eq; parsed.meet.rows_skipped, 0);
    let result = &parsed.meet.events[0].rows[0];
    check!(eq; result.place, Some(1));
    check!(eq; result.name, "SURFACE, Luke");
    check!(eq; result.school, "District 5-12");
    check!(eq; result.grade.map(|grade| grade.get()), Some(11));
    check!(eq;
        result.mark,
        Mark::TimeSeconds(crate::hytek::parse_time(MARK).ok_or("published mark")?)
    );
    check!(eq; result.heat.as_deref(), Some("8"));
    Ok(())
}

#[test]
fn a_blank_declared_mark_cell_never_promotes_school_digits() -> TestResult {
    const SCHOOL: &str = "District 5-12 Academy";
    const SPILLED: &str = "District 5-12 Academy 2024";
    let blank = format!(
        "   1 {:<20} {:<3} {:<24}{:>9} 8 (1)",
        "SURFACE, Luke", "JR", SCHOOL, ""
    );
    let blank_block = format!("Boys 3200M\n{}\n{blank}\n", header(20, 24));
    let parsed = page(&blank_block)?;
    check!(eq; parsed.meet.events[0].rows.len(), 0);
    check!(eq; parsed.meet.rows_parsed, 0);
    check!(eq; parsed.meet.rows_skipped, 1);
    check!(eq; parsed.skipped.len(), 1);
    check!(
        parsed.skipped[0].contains("missing or malformed mark"),
        "skipped: {:?}",
        parsed.skipped
    );
    let spilled = format!(
        "   1 {:<20} {:<3} {:<24}{:>9} 8 (1)",
        "SURFACE, Luke", "JR", SPILLED, ""
    );
    let spilled_block = format!("Boys 3200M\n{}\n{spilled}\n", header(20, 24));
    let spilled_page = page(&spilled_block)?;
    check!(eq; spilled_page.meet.events[0].rows.len(), 0);
    check!(eq; spilled_page.meet.rows_parsed, 0);
    check!(eq; spilled_page.skipped.len(), 1);
    check!(
        spilled_page.skipped[0].contains("row did not fit the column map"),
        "skipped: {:?}",
        spilled_page.skipped
    );
    Ok(())
}

#[test]
fn escaped_section_labels_decode_before_publication() -> TestResult {
    let line = row(20, 20, "SURFACE, Luke", "JR", "North Buncombe", "44.73");
    let block = format!("Girls&#039; Javelin\n{}\n{line}\n", header(20, 20));
    let parsed = page(&block)?;
    check!(eq; parsed.skipped, Vec::<String>::new());
    check!(eq; parsed.grade_issues, Vec::new());
    check!(eq; parsed.meet.rows_parsed, 1);
    check!(eq; parsed.meet.rows_skipped, 0);
    let event = &parsed.meet.events[0];
    check!(eq; event.label, "Girls' Javelin");
    check!(eq; event.gender, Gender::Girls);
    check!(eq; event.kind, EventKind::Javelin);
    check!(eq; event.rows.len(), 1);
    check!(eq; event.rows[0].school, "North Buncombe");
    check!(matches!(event.rows[0].mark, Mark::DistanceMetres(_)));
    check!(eq; event.rows[0].heat.as_deref(), Some("8"));
    Ok(())
}

#[test]
fn the_nc_wrapped_finals_capture_keeps_category_and_no_mark_rows() -> TestResult {
    let capture =
        include_str!("../../../tests/fixtures/milesplit/nc_meet_684812_rs1283641_raw.html");
    let parsed = parse_raw(capture, URL)?;
    check!(eq; parsed.skipped, Vec::<String>::new());
    let outside_high_school = |ordinal: u32, byte_offset: usize| RawGradeIssue {
        row: SourceRowLocator {
            ordinal,
            byte_offset,
            byte_length: 70,
        },
        raw_token: "8".to_string(),
        kind: RawGradeIssueKind::OutsideHighSchool,
    };
    check!(eq;
        parsed.grade_issues,
        vec![outside_high_school(25, 45385), outside_high_school(207, 58307)]
    );
    check!(eq; parsed.meet.rows_parsed, 214);
    check!(eq; parsed.meet.rows_skipped, 0);
    check!(eq; parsed.meet.events.len(), 1);
    let event = &parsed.meet.events[0];
    check!(event.label.ends_with("Boys 3200M Finals"));
    check!(eq; event.round.as_deref(), Some("Finals"));
    check!(eq; event.gender, Gender::Boys);
    check!(eq; event.kind, EventKind::Track3200m);
    check!(eq; event.rows.len(), 214);
    let named = |name: &str| {
        event
            .rows
            .iter()
            .find(|row| row.name == name)
            .ok_or("retained row")
    };
    let ferguson = named("FERGUSON, Michael")?;
    check!(eq; ferguson.school, "North Buncombe");
    check!(eq; ferguson.place, Some(1));
    check!(eq; ferguson.heat.as_deref(), Some("8"));
    check!(eq;
        ferguson.mark,
        Mark::TimeSeconds(crate::hytek::parse_time("8:44.73").ok_or("published mark")?)
    );
    for name in ["WHARTON, Elijah", "WILLCOX, Jack"] {
        check!(eq; named(name)?.mark, Mark::Raw("DNF".to_string()));
    }
    for name in ["JENKINS, Grady", "TEMPLETON, John"] {
        let row = named(name)?;
        check!(eq; row.mark, Mark::Raw("NT".to_string()));
        check!(eq; row.heat.as_deref(), Some("7"));
        check!(eq; row.place, None);
    }
    check!(eq; event.rows.last().ok_or("last row")?.name, "TEMPLETON, John");
    Ok(())
}
