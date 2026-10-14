use super::{
    parse_list_page, parse_list_path, parse_team_page, published_date, Absorb, ListContext, Page,
    SchoolIndex, SourceRef, TestResult, UsJurisdiction, LIST, LIVE_LIST, ROSTER,
};

const URL: &str = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
const ACQUIRED: &str = "2026-09-22T03:55:31Z";

#[test]
fn published_single_days_preserve_raw_labels_and_real_calendar_dates() -> TestResult {
    for (label, iso) in [
        ("Mar 28, 2026", "2026-03-28"),
        ("March 28, 2026", "2026-03-28"),
        ("MAR. 28 2026", "2026-03-28"),
        ("Sept 1, 2026", "2026-09-01"),
        ("September 12, 2026", "2026-09-12"),
        ("Feb 29, 2024", "2024-02-29"),
        ("Feb 29, 2000", "2000-02-29"),
        ("Jan 1, 1900", "1900-01-01"),
        ("Dec 31, 2999", "2999-12-31"),
    ] {
        let date = published_date(label).ok_or("valid published single day")?;
        check!(eq; date.raw.as_str(), label);
        check!(eq; date.iso.as_str(), iso);
    }
    Ok(())
}

#[test]
fn ambiguous_or_malformed_labels_cannot_become_exact_single_days() -> TestResult {
    for label in [
        "Mar 28?, 2026",
        "Mar 28-29, 2026",
        "Mar 28/29, 2026",
        "Mar 28, 2026 or Mar 29, 2026",
        "Mar 28, 2026 (updated Sep 22, 2026)",
        "Marching 28, 2026",
        "Mar 28,, 2026",
        "Mar 28, 2026,",
        "Mar +28, 2026",
        "Mar 028, 2026",
        "Mar 28, +2026",
        "Feb 29, 2025",
        "Feb 30, 2026",
        "Apr 31, 2026",
        "Feb 29, 1900",
        "Feb 29, 2100",
        "Mar 0, 2026",
        "Mar 32, 2026",
        "Dec 31, 1899",
        "Jan 1, 3000",
        "",
    ] {
        check!(eq; published_date(label), None, "{label:?}");
    }
    Ok(())
}

#[test]
fn captured_list_dates_remain_supported_without_using_acquisition_time() -> TestResult {
    for (html, expected_rows) in [(LIST, 28_usize), (LIVE_LIST, 2_usize)] {
        let parsed = parse_list_page(html);
        let rows: Vec<_> = parsed
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .collect();
        check!(eq; rows.len(), expected_rows);
        for row in rows {
            let date = row.date.as_ref().ok_or("captured row date")?;
            check!(date.iso.starts_with("2026-03-"));
            check!(date.raw.starts_with("Mar "));
        }
    }
    let source = SourceRef::new("tfrrs_in", Some(URL.to_string()));
    let route = parse_list_path(URL).ok_or("list route")?;
    let index = SchoolIndex::from_schools(&[]);
    let mut absorb = Absorb::new(&index);
    absorb.absorb_list(&context(&source, &route)?, &parse_list_page(LIST))?;
    check!(eq; absorb.accumulator.performances.len(), 20);
    for performance in absorb.accumulator.performances.values() {
        check!(performance.date.starts_with("2026-03-"));
        check!(!performance.evidence.is_empty());
        check!(performance
            .evidence
            .iter()
            .all(|evidence| { evidence.observed_on == ACQUIRED && evidence.source == source }));
    }
    Ok(())
}

#[test]
fn ambiguous_row_dates_keep_projection_unfinished_and_other_rows_available() -> TestResult {
    for label in [
        "Mar 28?, 2026",
        "Mar 28-29, 2026",
        "Mar 28, 2026 or Mar 29, 2026",
    ] {
        let html = LIST.replace("Mar 28, 2026", label);
        let source = SourceRef::new("tfrrs_in", Some(URL.to_string()));
        let route = parse_list_path(URL).ok_or("list route")?;
        let index = SchoolIndex::from_schools(&[]);
        let mut absorb = Absorb::new(&index);
        let outcome = absorb.absorb_list(&context(&source, &route)?, &parse_list_page(&html));
        check!(matches!(
            outcome,
            Err(crate::CrawlError::PerformanceDateUnknown { .. })
        ));
        check!(eq; absorb.accumulator.performances.len(), 16);
        check!(eq; absorb.stats.rows_seen, 28);
        check!(eq; absorb.stats.rows_relay, 5);
        check!(eq; absorb.stats.rows_absorbed, 16);
        check!(absorb.accumulator.performances.values().all(|performance| {
            performance.date != "2026-03-28" && performance.date != "2026-09-22"
        }));
        check!(html.contains(label));
    }
    Ok(())
}

#[test]
fn unrelated_roster_program_discovery_does_not_require_meet_history_days() -> TestResult {
    check!(ROSTER.contains("June 20-21, 2026"));
    check!(ROSTER.contains("June 21-22, 2025"));
    let expected = parse_team_page(ROSTER);
    check!(!expected.athletes.is_empty());
    let unresolved = ROSTER
        .replace("June 20-21, 2026", "Unknown meet date")
        .replace("June 21-22, 2025", "Date to be confirmed");
    check!(eq; parse_team_page(&unresolved), expected);
    Ok(())
}

#[test]
fn ambiguous_or_malformed_date_cells_cannot_be_truncated_to_an_exact_day() -> TestResult {
    for label in [
        "Mar 28, 2026<div>or Mar 29, 2026</div>",
        "Mar 28, 2026<div><span>?</span></div>",
        "Mar 28, 2026<DIV>junk</DIV>",
        "Mar 28, 2026<span> or Mar 29, 2026</span>",
        "Mar 28, 2026 &lt; 2027",
        "Mar 28, 2026</div><div data-label=\"Meet Date\">Mar 29, 2026",
        "Mar 28, 2026</div><div data-label=\"Meet Date\">Mar 28, 2026",
        "<span>Mar 28, 2026",
        "<span>Mar 28, 2026</b>",
        "Mar 28, 2026</span>",
    ] {
        let html = LIST.replace("Mar 28, 2026", label);
        let parsed = parse_list_page(&html);
        let rows: Vec<_> = parsed
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .collect();
        check!(eq; rows.len(), 28);
        check!(eq; rows.iter().filter(|row| row.date.is_none()).count(), 6);
        let expected = parse_list_page(LIST);
        let expected_rows = expected.sections.iter().flat_map(|section| &section.rows);
        for (row, original) in rows.iter().zip(expected_rows) {
            let mut restored = (**row).clone();
            restored.date = original.date.clone();
            check!(eq; restored, *original);
        }
        let source = SourceRef::new("tfrrs_in", Some(URL.to_string()));
        let route = parse_list_path(URL).ok_or("list route")?;
        let index = SchoolIndex::from_schools(&[]);
        let mut absorb = Absorb::new(&index);
        let result = absorb.absorb_list(&context(&source, &route)?, &parsed);
        match result {
            Err(crate::CrawlError::PerformanceDateUnknown { published, as_of }) => {
                check!(eq; published.as_str(), "");
                check!(eq; as_of, context(&source, &route)?.page.performance_as_of);
            }
            other => return Err(format!("expected unknown date, got {other:?}").into()),
        }
        check!(eq; absorb.accumulator.performances.len(), 16);
        check!(eq; (absorb.stats.rows_seen, absorb.stats.rows_relay, absorb.stats.rows_absorbed), (28, 5, 16));
    }
    Ok(())
}

#[test]
fn nested_single_day_markup_preserves_every_published_date() -> TestResult {
    let expected = parse_list_page(LIST);
    for label in [
        "<span>Mar</span> 28, 2026",
        "<div>Mar 28, 2026</div>",
        "Mar&#160;28,&#160;2026",
        "<span>Ma</span>r 2<b>8</b>, 20<i>26</i>",
    ] {
        let html = LIST.replace("Mar 28, 2026", label);
        check!(eq; parse_list_page(&html), expected);
    }
    Ok(())
}

fn context<'a>(
    source: &'a SourceRef,
    route: &'a crate::tfrrs::parse::ListPath,
) -> TestResult<ListContext<'a>> {
    Ok(ListContext {
        page: Page {
            source,
            observed_on: ACQUIRED,
            performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 14).ok_or("horizon")?,
            jurisdiction: UsJurisdiction::Indiana,
        },
        list: route,
        filter: None,
    })
}
