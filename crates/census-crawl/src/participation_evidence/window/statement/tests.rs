use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const VALID: &str = "Full documented participation window | school year: 2025-26 | cohort: 2027 | from: 2026-03-01 | through: 2026-05-31 | documented summer boundary: 2026-05-31";

#[test]
fn explicit_full_window_preserves_every_declared_boundary() -> TestResult {
    let boundary = parse(VALID)?;
    check!(eq; boundary.from.as_str(), "2026-03-01");
    check!(eq; boundary.through.as_str(), "2026-05-31");
    check!(eq; boundary.documented_summer_end.as_str(), "2026-05-31");
    check!(eq; boundary.school_year.get(), 2025);
    check!(eq; boundary.cohort.get(), 2027);
    Ok(())
}

#[test]
fn school_year_notation_preserves_century_endpoint() -> TestResult {
    check!(eq; school_year("1999-00")?.get(), 1999);
    check!(matches!(
        school_year("1999-99"),
        Err(StoreError::Refused { .. })
    ));
    Ok(())
}

#[test]
fn partial_or_incoherent_boundaries_cannot_form_a_full_window() -> TestResult {
    for (original, replacement) in [
        ("Full documented participation window", "In-season dates"),
        ("school year: 2025-26", "school year: 2025-27"),
        ("cohort: 2027", "cohort: 2050"),
        ("from: 2026-03-01", "from: 2026-06-01"),
        (
            "documented summer boundary: 2026-05-31",
            "documented summer boundary: 2026-06-30",
        ),
        (" | documented summer boundary: 2026-05-31", ""),
        ("school year: 2025-26", "cohort: 2027"),
    ] {
        check!(matches!(
            parse(&VALID.replacen(original, replacement, 1)),
            Err(StoreError::Refused { .. })
        ));
    }
    check!(matches!(
        parse(&format!("{VALID} | through: 2026-06-30")),
        Err(StoreError::Refused { .. })
    ));
    Ok(())
}
