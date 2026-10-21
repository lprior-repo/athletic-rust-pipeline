use super::super::*;

#[test]
fn duplicate_topology_attributes_refuse_the_exact_shifted_original_row() -> TestResult {
    for (from, to) in [
        (
            "<div class=\"row gender_m standard_event_hnd_61\">",
            "<div class=\"row gender_m standard_event_hnd_61\" class=\"gender_f\">",
        ),
        (
            "<div class=\"performance-list-body \">",
            "<div class=\"performance-list-body \" class=\"other\">",
        ),
        (
            "<h3 class=\"font-weight-500\">",
            "<h3 class=\"font-weight-500\" class=\"other\">",
        ),
    ] {
        let (capture, row) = fixture::prefix_replaced(from, to)?;
        check!(matches!(
            read_retained_tfrrs_list_rows(&capture, &[row]),
            Err(RetainedResultOwnershipError::Invalid(
                "malformed TFRRS owned topology tag"
            ))
        ));
    }
    Ok(())
}
use super::fixture::{self, TestResult, BODY};

#[test]
fn selected_hidden_inert_and_foreign_markup_rows_have_no_active_ownership() -> TestResult {
    for attribute in [
        "hidden",
        "inert",
        "aria-hidden=\"true\"",
        "style=\"display:none\"",
        "style=\"visibility:hidden\"",
    ] {
        let replacement = format!("<div {attribute} class=");
        let (capture, row) = fixture::first_replaced("<div class=", &replacement)?;
        check!(read_retained_tfrrs_list_rows(&capture, &[row]).is_err());
    }
    for container in ["template", "svg", "math", "noscript", "iframe"] {
        let (capture, mut row) =
            fixture::changed_row(|row| format!("<{container}>{row}</{container}>"))?;
        row.byte_offset = row
            .byte_offset
            .checked_add(container.len() + 2)
            .ok_or("fixture offset")?;
        row.byte_length = fixture::locators()[0].byte_length;
        check!(read_retained_tfrrs_list_rows(&capture, &[row]).is_err());
    }
    Ok(())
}

#[test]
fn duplicate_cells_native_links_and_unbalanced_selected_markup_are_rejected() -> TestResult {
    for (from, to) in [
        ("data-label=\"Year\">JR</div>", "data-label=\"Year\">JR</div><div data-label=\"Year\">JR</div>"),
        (">Justin Reckers</a>", ">Justin Reckers</a><a href=\"/athletes/8650211/Columbus_North/_Justin_Reckers\">Justin Reckers</a>"),
        ("9:15.75</a> <span>#</span>", "9:15.75</a> <span>#"),
        ("<span title=", "<span hidden title="),
    ] {
        let (capture, row) = fixture::first_replaced(from, to)?;
        check!(read_retained_tfrrs_list_rows(&capture, &[row]).is_err());
    }
    Ok(())
}

#[test]
fn foreign_links_wrong_meet_grade_date_and_conversion_kind_cannot_bind() -> TestResult {
    for (from, to) in [
        (
            "https://indiana.tfrrs.org/athletes/8650211",
            "https://foreign.invalid/athletes/8650211",
        ),
        ("/results/95201/5862520/", "/results/93917/5862520/"),
        (
            "/teams/tf/Columbus_North_m.html",
            "/teams/tf/Columbus_North_f.html",
        ),
        ("data-label=\"Year\">JR", "data-label=\"Year\">8"),
        ("Mar 21, 2026", "Feb 30, 2026"),
        ("Mar 21, 2026", "Mar 21, 2025"),
        ("for Track Size.", "for Altitude."),
        ("9:15.75</a>", "unmeasured</a>"),
    ] {
        let (capture, row) = fixture::first_replaced(from, to)?;
        check!(read_retained_tfrrs_list_rows(&capture, &[row]).is_err());
    }
    Ok(())
}

#[test]
fn conversion_note_outside_the_published_mark_wrapper_is_rejected() -> TestResult {
    let (capture, row) = fixture::changed_row(|row| {
        row.replacen(
            "<span title=\"Converted from 9:22.21 for Track Size. \">",
            "<span title=\"Converted from 9:22.21 for Track Size. \"></span><span>",
            1,
        )
    })?;
    check!(read_retained_tfrrs_list_rows(&capture, &[row]).is_err());
    Ok(())
}

#[test]
fn unclosed_section_and_nested_section_context_are_rejected() -> TestResult {
    let html = std::str::from_utf8(BODY)?;
    let row = fixture::locators()[0];
    let end = row
        .byte_offset
        .checked_add(row.byte_length)
        .ok_or("row end")?;
    let truncated = fixture::capture(html.get(..end).ok_or("truncated document")?.as_bytes())?;
    check!(read_retained_tfrrs_list_rows(&truncated, &[row]).is_err());
    let (capture, row) = fixture::changed_row(|row| {
        row.replacen(
            "<div class=",
            "<div class=\"row gender_m standard_event_hnd_61\"><div class=",
            1,
        )
    })?;
    check!(read_retained_tfrrs_list_rows(&capture, &[row]).is_err());
    Ok(())
}

#[test]
fn malformed_excerpt_cannot_substitute_for_the_complete_genuine_capture() -> TestResult {
    let body = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/tfrrs/indiana_list_5489_excerpt.html"
    ));
    let capture = fixture::capture(body)?;
    let html = std::str::from_utf8(body)?;
    let at = html.find("_Justin_Reckers").ok_or("excerpt Justin")?;
    let start = html
        .get(..at)
        .and_then(|prefix| prefix.rfind("<div class=\"performance-list-row"))
        .ok_or("excerpt row start")?;
    let row = SourceRowLocator {
        ordinal: 6,
        byte_offset: start,
        byte_length: 1409,
    };
    check!(read_retained_tfrrs_list_rows(&capture, &[row]).is_err());
    Ok(())
}
