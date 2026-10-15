use super::{cells, page, parse, refusal, TestResult};
use crate::mhsaa_enrollment::EnrollmentParseError;

#[test]
fn utf8_names_internal_spacing_and_numeric_name_parts_are_preserved() -> TestResult {
    let text = page(
        "2026-27",
        "1 10 École  du Lac 🦀 123 0000 0001\n2 11 School 42 7 0\n",
    );
    let document = parse(&text)?;
    check!(eq; document.schools().len(), 2);
    let first = document.schools().first().ok_or("missing Unicode school")?;
    check!(eq; cells(first), (1, 10, "École  du Lac 🦀 123", 0, 1, 1));
    let second = document
        .schools()
        .get(1)
        .ok_or("missing numeric-name school")?;
    check!(eq; cells(second), (2, 11, "School 42", 7, 0, 1));
    Ok(())
}

#[test]
fn published_zero_and_maximum_unsigned_enrollments_are_actual_values() -> TestResult {
    let document = parse(&page("2026-27", "0001 4294967295 School 0000 4294967295\n"))?;
    let school = document.schools().first().ok_or("missing school")?;
    check!(eq; cells(school), (1, u32::MAX, "School", 0, u32::MAX, 1));
    Ok(())
}

#[test]
fn front_numeric_fields_require_nonzero_unsigned_ascii_and_no_overflow() -> TestResult {
    for ordinal in ["0", "+1", "-1", "1x", "1.0", "１", "65536", "4294967296"] {
        let text = page("2026-27", &format!("{ordinal} 10 School 1 1\n"));
        check!(
            matches!(
                refusal(&text)?,
                EnrollmentParseError::Row {
                    page: 1,
                    field: "ordinal"
                }
            ),
            "{ordinal}"
        );
    }
    for id in ["0", "+10", "-10", "10x", "10.0", "１０", "4294967296"] {
        let text = page("2026-27", &format!("1 {id} School 1 1\n"));
        check!(
            matches!(
                refusal(&text)?,
                EnrollmentParseError::Row {
                    page: 1,
                    field: "school id"
                }
            ),
            "{id}"
        );
    }
    Ok(())
}

#[test]
fn back_numeric_fields_require_whole_unsigned_ascii_tokens() -> TestResult {
    for token in ["abc", "+1", "-1", "1x", "1.0", "1,000", "１", "4294967296"] {
        let total = page("2026-27", &format!("1 10 School {token} 1\n"));
        check!(
            matches!(
                refusal(&total)?,
                EnrollmentParseError::Row {
                    page: 1,
                    field: "total enrollment"
                }
            ),
            "{token}"
        );
        let classification = page("2026-27", &format!("1 10 School 1 {token}\n"));
        check!(
            matches!(
                refusal(&classification)?,
                EnrollmentParseError::Row {
                    page: 1,
                    field: "classification enrollment"
                }
            ),
            "{token}"
        );
    }
    Ok(())
}

#[test]
fn missing_fields_and_wrapped_physical_rows_are_refused() -> TestResult {
    for (body, field) in [
        ("1\n", "school id"),
        ("1 10\n", "classification enrollment"),
        ("1 10 1\n", "total enrollment"),
        ("1 10 1 1\n", "school name"),
        ("1 10 School 1\n", "total enrollment"),
        ("1 10 School\n1 1\n", "classification enrollment"),
        ("1 10 School 1 1 extra\n", "classification enrollment"),
    ] {
        check!(
            matches!(refusal(&page("2026-27", body))?, EnrollmentParseError::Row { page: 1, field: actual }
        if actual == field),
            "{body}"
        );
    }
    Ok(())
}

#[test]
fn malformed_late_rows_and_unknown_numeric_rows_are_not_silently_lost() -> TestResult {
    for (line, field) in [
        ("School without ordinal 1 1", "ordinal"),
        ("Class A 783 and above", "ordinal"),
        ("104 105", "classification enrollment"),
        ("2 20 School abc 2", "total enrollment"),
    ] {
        let text = page(
            "2026-27",
            &format!("1 10 First 1 1\n{line}\n3 30 Later 3 3\n"),
        );
        check!(
            matches!(refusal(&text)?, EnrollmentParseError::Row { page: 1, field: actual }
        if actual == field),
            "{line}"
        );
    }
    Ok(())
}

#[test]
fn zero_id_on_later_page_returns_a_typed_row_error() -> TestResult {
    let text = format!(
        "{}\u{000c}{}",
        page("2026-27", "1 10 First 1 1\n"),
        page("2026-27", "2 0 Second 2 2\n")
    );
    check!(matches!(
        refusal(&text)?,
        EnrollmentParseError::Row {
            page: 2,
            field: "school id"
        }
    ));
    Ok(())
}
