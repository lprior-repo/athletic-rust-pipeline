use super::{page, parse, refusal, year, TestResult, COLUMNS};
use crate::mhsaa_enrollment::{parse_enrollment_text, EnrollmentParseError};

#[test]
fn exact_official_title_requires_adjacent_valid_academic_end_year() -> TestResult {
    for title in [
        "2026-99 MHSAA Enrollment List",
        "2026-2027 MHSAA Enrollment List",
        "2026-7 MHSAA Enrollment List",
        "+2026-27 MHSAA Enrollment List",
        "２０２６-27 MHSAA Enrollment List",
        "2026-+27 MHSAA Enrollment List",
        "2026--27 MHSAA Enrollment List",
        "1899-00 MHSAA Enrollment List",
        "2101-02 MHSAA Enrollment List",
        "2026-27 MHSAA Enrollment List unexpected",
        "prefix 2026-27 MHSAA Enrollment List",
        "2026-27 Unofficial Enrollment List",
    ] {
        let text = format!("{title}\n{COLUMNS}1 10 School 1 1\n");
        check!(
            matches!(refusal(&text)?, EnrollmentParseError::MissingHeader),
            "{title}"
        );
    }
    Ok(())
}

#[test]
fn year_boundaries_and_century_rollovers_preserve_declared_year() -> TestResult {
    for (start, academic) in [
        (1900, "1900-01"),
        (1999, "1999-00"),
        (2099, "2099-00"),
        (2100, "2100-01"),
    ] {
        let expected = year(start)?;
        let document = parse_enrollment_text(&page(academic, "1 10 School 1 1\n"), expected)?;
        check!(eq; document.school_year(), expected);
        check!(eq; document.schools().len(), 1);
    }
    Ok(())
}

#[test]
fn every_page_requires_its_own_matching_title_and_table_headers() -> TestResult {
    let first = page("2026-27", "1 10 First 1 1\n");
    let wrong = format!("{first}\u{000c}{}", page("2025-26", "2 20 Second 2 2\n"));
    check!(
        matches!(refusal(&wrong)?, EnrollmentParseError::SchoolYear { expected, found }
    if expected == year(2026)? && found == year(2025)?)
    );
    let missing = format!("{first}\u{000c}{COLUMNS}2 20 Second 2 2\n");
    check!(matches!(
        refusal(&missing)?,
        EnrollmentParseError::MissingHeader
    ));
    let missing_columns =
        format!("{first}\u{000c}2026-27 MHSAA Enrollment List\n2 20 Second 2 2\n");
    check!(matches!(
        refusal(&missing_columns)?,
        EnrollmentParseError::Layout { page: 2 }
    ));
    Ok(())
}

#[test]
fn missing_reordered_and_truncated_column_layouts_are_refused() -> TestResult {
    for layout in [
        "",
        "ID School Total Classification\n",
        "Enrollment Enrollment\n",
        "Enrollment\nID School Total Classification\n",
        "Enrollment Enrollment\nID School Classification Total\n",
        "Enrollment Enrollment\nID School Total\n",
        "Enrollment Enrollment\nID School Total Classification Extra\n",
        "Enrollment Enrollment\n1 10 School 1 1\nID School Total Classification\n",
    ] {
        let text = format!("2026-27 MHSAA Enrollment List\n{layout}1 10 School 1 1\n");
        check!(
            matches!(refusal(&text)?, EnrollmentParseError::Layout { page: 1 }),
            "{layout}"
        );
    }
    Ok(())
}

#[test]
fn absent_or_empty_physical_page_is_not_silently_discarded() -> TestResult {
    for text in ["", "\n\t\n", "arbitrary text\n1 10 School 1 1\n"] {
        check!(matches!(
            refusal(text)?,
            EnrollmentParseError::MissingHeader
        ));
    }
    let text = format!(
        "{}\u{000c}\u{000c}{}",
        page("2026-27", "1 10 School 1 1\n"),
        page("2026-27", "2 20 School 1 1\n")
    );
    check!(matches!(
        refusal(&text)?,
        EnrollmentParseError::MissingHeader
    ));
    Ok(())
}

#[test]
fn recognized_official_preamble_and_crlf_preserve_school_rows() -> TestResult {
    let text = "\r\n 2026-27 MHSAA Enrollment List \r\n\
        Procedures for determining classification are outlined on pages 104 - 105 of the MHSAA Handbook\r\n\
        Class A 783 and above\r\nClass B 782 – 358\r\nClass C 357– 167\r\nClass D 166 and below\r\n\
        Enrollment Enrollment\r\nID School Total Classification\r\n1 10 School 1 2\r\n";
    let document = parse(text)?;
    check!(eq; document.schools().len(), 1);
    let school = document.schools().first().ok_or("missing school")?;
    check!(eq; (
        school.total_enrollment(),
        school.classification_enrollment()
    ),
    (1, 2));
    Ok(())
}

#[test]
fn unknown_preamble_and_repeated_headers_cannot_hide_rows() -> TestResult {
    let unknown = format!(
        "2026-27 MHSAA Enrollment List\nunknown 1 10 School 1 1\n{COLUMNS}1 10 School 1 1\n"
    );
    check!(matches!(
        refusal(&unknown)?,
        EnrollmentParseError::Layout { page: 1 }
    ));
    let repeated = page(
        "2026-27",
        "1 10 First 1 1\nID School Total Classification\n2 20 Second 2 2\n",
    );
    check!(matches!(
        refusal(&repeated)?,
        EnrollmentParseError::Row {
            page: 1,
            field: "ordinal"
        }
    ));
    Ok(())
}
