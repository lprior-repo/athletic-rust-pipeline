use super::{page, parse, refusal, TestResult, COLUMNS};
use crate::mhsaa_enrollment::{
    EnrollmentParseError, MAX_ENROLLMENT_LINE_BYTES, MAX_ENROLLMENT_NAME_BYTES,
    MAX_ENROLLMENT_PAGES, MAX_ENROLLMENT_ROWS, MAX_ENROLLMENT_TEXT_BYTES,
};
use std::fmt::Write;

fn limit(text: &str, field: &'static str, maximum: usize) -> TestResult {
    assert!(
        matches!(refusal(text)?, EnrollmentParseError::Limit { field: actual, maximum: bound }
        if actual == field && bound == maximum),
        "{field}"
    );
    Ok(())
}

#[test]
fn text_admission_accepts_one_mebibyte_and_refuses_the_next_byte() -> TestResult {
    let mut text = page("2026-27", "1 10 School 0 0\n");
    let padding = MAX_ENROLLMENT_TEXT_BYTES
        .checked_sub(text.len())
        .ok_or("padding underflow")?;
    text.push_str(&"\n".repeat(padding));
    assert_eq!(text.len(), MAX_ENROLLMENT_TEXT_BYTES);
    assert_eq!(parse(&text)?.schools().len(), 1);
    text.push('\n');
    limit(&text, "text bytes", MAX_ENROLLMENT_TEXT_BYTES)
}

#[test]
fn line_admission_includes_title_preamble_columns_and_rows_before_parsing() -> TestResult {
    let title = "2026-27 MHSAA Enrollment List";
    let padding = MAX_ENROLLMENT_LINE_BYTES
        .checked_sub(title.len())
        .ok_or("padding underflow")?;
    let mut text = format!("{}{title}\n{COLUMNS}1 10 School 1 1\n", " ".repeat(padding));
    assert_eq!(parse(&text)?.schools().len(), 1);
    text.insert(0, ' ');
    limit(&text, "line bytes", MAX_ENROLLMENT_LINE_BYTES)?;
    let oversize = " ".repeat(
        MAX_ENROLLMENT_LINE_BYTES
            .checked_add(1)
            .ok_or("line overflow")?,
    );
    for text in [
        format!("{oversize}invalid title\n"),
        format!("{title}\n{oversize}Procedures\n{COLUMNS}1 10 School 1 1\n"),
        format!("{title}\n{oversize}Enrollment Enrollment\nID School Total Classification\n1 10 School 1 1\n"),
        page("2026-27", &format!("1 10 {oversize}School 1 1\n")),
    ] {
        limit(&text, "line bytes", MAX_ENROLLMENT_LINE_BYTES)?;
    }
    Ok(())
}

#[test]
fn unicode_name_admission_is_in_bytes_and_precedes_owned_name_growth() -> TestResult {
    let name = "é".repeat(128);
    assert_eq!(name.len(), MAX_ENROLLMENT_NAME_BYTES);
    let document = parse(&page("2026-27", &format!("1 10 {name} 1 1\n")))?;
    let school = document.schools().first().ok_or("missing school")?;
    assert_eq!(school.name(), name);
    limit(
        &page("2026-27", &format!("1 10 {name}x 1 1\n")),
        "name bytes",
        MAX_ENROLLMENT_NAME_BYTES,
    )
}

#[test]
fn row_admission_accepts_4096_and_refuses_before_parsing_a_4097th_row() -> TestResult {
    let mut rows = String::new();
    for ordinal in 1..=MAX_ENROLLMENT_ROWS {
        writeln!(rows, "{ordinal} {ordinal} School 0 0")?;
    }
    let text = page("2026-27", &rows);
    let document = parse(&text)?;
    assert_eq!(document.schools().len(), MAX_ENROLLMENT_ROWS);
    assert_eq!(
        usize::from(
            document
                .schools()
                .last()
                .ok_or("missing last school")?
                .ordinal()
        ),
        MAX_ENROLLMENT_ROWS
    );
    rows.push_str("not even a valid numeric row\n");
    limit(&page("2026-27", &rows), "rows", MAX_ENROLLMENT_ROWS)
}

#[test]
fn page_admission_accepts_64_with_a_trailing_terminator_not_a_phantom_page() -> TestResult {
    let mut text = String::new();
    for ordinal in 1..=MAX_ENROLLMENT_PAGES {
        text.push_str(&page(
            "2026-27",
            &format!("{ordinal} {ordinal} School 1 1\n"),
        ));
        text.push('\u{000c}');
    }
    let document = parse(&text)?;
    assert_eq!(document.schools().len(), MAX_ENROLLMENT_PAGES);
    assert_eq!(
        usize::from(
            document
                .schools()
                .last()
                .ok_or("missing last school")?
                .page()
        ),
        MAX_ENROLLMENT_PAGES
    );
    let unterminated = text
        .strip_suffix('\u{000c}')
        .ok_or("missing page terminator")?;
    assert_eq!(parse(unterminated)?, document);
    text.push_str("invalid title on physical page 65");
    limit(&text, "pages", MAX_ENROLLMENT_PAGES)
}

#[test]
fn overlong_later_page_line_is_refused_before_header_interpretation() -> TestResult {
    let text = format!(
        "{}\u{000c}{}",
        page("2026-27", "1 10 School 1 1\n"),
        "x".repeat(
            MAX_ENROLLMENT_LINE_BYTES
                .checked_add(1)
                .ok_or("line overflow")?
        )
    );
    limit(&text, "line bytes", MAX_ENROLLMENT_LINE_BYTES)
}
