use super::super::{EnrollmentDocument, EnrollmentParseError, EnrollmentSchool};
use super::parse_enrollment_text;
use census_domain::model::SchoolYear;
use std::collections::BTreeSet;

#[path = "parse_tests/bounds.rs"]
mod bounds;
#[path = "parse_tests/headers.rs"]
mod headers;
#[path = "parse_tests/rows.rs"]
mod rows;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const OFFICIAL_TEXT: &str = include_str!("../../tests/fixtures/mhsaa_enrollment/enrollment.txt");
const COLUMNS: &str =
    "Enrollment                   Enrollment\nID School    Total    Classification\n";

fn page(year: &str, rows: &str) -> String {
    format!("{year} MHSAA Enrollment List\n{COLUMNS}{rows}")
}

fn parse(text: &str) -> TestResult<EnrollmentDocument> {
    Ok(parse_enrollment_text(text, SchoolYear::DEFAULT)?)
}

fn refusal(text: &str) -> TestResult<EnrollmentParseError> {
    match parse_enrollment_text(text, SchoolYear::DEFAULT) {
        Err(error) => Ok(error),
        Ok(document) => {
            Err(format!("unexpectedly accepted {} rows", document.schools().len()).into())
        }
    }
}

fn year(start: i16) -> TestResult<SchoolYear> {
    SchoolYear::new(start).ok_or_else(|| "invalid test school year".into())
}

fn cells(school: &EnrollmentSchool) -> (u16, u32, &str, u32, u32, u16) {
    (
        school.ordinal(),
        school.mhsaa_id(),
        school.name(),
        school.total_enrollment(),
        school.classification_enrollment(),
        school.page(),
    )
}

#[test]
fn official_text_preserves_known_cells_and_every_published_ordinal() -> TestResult {
    let document = parse(OFFICIAL_TEXT)?;
    check!(eq; document.school_year(), SchoolYear::DEFAULT);
    check!(eq; document.physical_pages(), 40);
    check!(eq; document.schools().len(), 755);
    let first = document.schools().first().ok_or("missing first school")?;
    check!(eq; cells(first), (1, 5792, "East Kentwood", 3042, 3042, 1));
    let central = document
        .schools()
        .iter()
        .find(|school| school.mhsaa_id() == 4638)
        .ok_or("missing Detroit Catholic Central")?;
    check!(eq; cells(central),
    (15, 4638, "Detroit Catholic Central", 1046, 2092, 1));
    let last = document.schools().last().ok_or("missing last school")?;
    check!(eq; cells(last),
    (755, 9962, "Wyoming West Michigan Lutheran", 11, 11, 20));
    let mut ids = BTreeSet::new();
    for (ordinal, school) in (1usize..=755).zip(document.schools()) {
        check!(eq; usize::from(school.ordinal()), ordinal);
        check!(
            ids.insert(school.mhsaa_id()),
            "duplicate ID at ordinal {ordinal}"
        );
        check!(eq; usize::from(school.page()),
        ordinal
            .checked_sub(1)
            .ok_or("ordinal underflow")?
            .checked_div(38)
            .ok_or("invalid page divisor")?
            .checked_add(1)
            .ok_or("page overflow")?);
    }
    Ok(())
}

#[test]
fn official_text_refuses_a_different_requested_year() -> TestResult {
    let expected = year(2025)?;
    let result = parse_enrollment_text(OFFICIAL_TEXT, expected);
    check!(
        matches!(result, Err(EnrollmentParseError::SchoolYear { expected: actual, found })
    if actual == expected && found == SchoolYear::DEFAULT)
    );
    Ok(())
}

#[test]
fn physical_pages_preserve_contiguous_rows_and_page_numbers() -> TestResult {
    let text = format!(
        "{}\u{000c}{}\u{000c}",
        page("2026-27", "1 5792 East Kentwood 3042 3042\n"),
        page("2026-27", "2 4638 Detroit Catholic Central 1046 2092\n")
    );
    let document = parse(&text)?;
    check!(eq; document.schools().len(), 2);
    let second = document.schools().get(1).ok_or("missing second school")?;
    check!(eq; cells(second),
    (2, 4638, "Detroit Catholic Central", 1046, 2092, 2));
    Ok(())
}

#[test]
fn ordinal_gaps_repeats_and_reordering_are_refused() -> TestResult {
    for (body, expected, found) in [
        ("2 10 School 1 1\n", 1, 2),
        ("1 10 One 1 1\n3 20 Three 1 1\n", 2, 3),
        ("1 10 One 1 1\n1 20 Repeat 1 1\n", 2, 1),
        ("1 10 One 1 1\n2 20 Two 1 1\n1 30 Reordered 1 1\n", 3, 1),
    ] {
        let error = refusal(&page("2026-27", body))?;
        check!(
            matches!(error, EnrollmentParseError::Ordinal { expected: actual, found: ordinal }
        if actual == expected && ordinal == found),
            "{body}"
        );
    }
    Ok(())
}

#[test]
fn duplicate_ids_are_refused_across_physical_pages() -> TestResult {
    let text = format!(
        "{}\u{000c}{}",
        page("2026-27", "1 5792 First 1 1\n"),
        page("2026-27", "2 5792 Second 2 2\n")
    );
    check!(matches!(
        refusal(&text)?,
        EnrollmentParseError::DuplicateSchool { id: 5792 }
    ));
    Ok(())
}

#[test]
fn declared_table_without_rows_is_not_a_zero_school_document() -> TestResult {
    check!(matches!(
        refusal(&page("2026-27", ""))?,
        EnrollmentParseError::NoSchools
    ));
    Ok(())
}
