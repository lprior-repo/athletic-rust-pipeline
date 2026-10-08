use census_domain::model::{CanonicalSchool, MEET_STATE_UNRESOLVED};
use census_domain::UsJurisdiction;

use crate::report::ReportResult;
use crate::workbook::cells::{row, Cell};

pub(super) const SCHOOL_WIDTHS: [u16; 24] = [
    16, 44, 10, 26, 14, 12, 12, 42, 42, 30, 10, 10, 20, 36, 28, 24, 12, 16, 28, 28, 28, 48, 20, 68,
];

pub(super) fn schools_sheet(schools: &[CanonicalSchool]) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = vec![school_header()];
    let mut sorted: Vec<&CanonicalSchool> = schools.iter().collect();
    sorted.sort_by(|left, right| {
        state_code(left)
            .cmp(state_code(right))
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.as_str().cmp(right.id.as_str()))
    });
    for school in sorted {
        cells.push(school_row(school)?);
    }
    Ok(cells)
}

fn school_header() -> Vec<Cell> {
    let mut cells = row!(
        "School ID",
        "School",
        "State",
        "City",
        "Association",
        "Class",
        "Enrollment",
        "Athletics site",
        "School site",
        "Aliases",
        "Sources",
        "Conflicts",
    );
    cells.extend(
        crate::export::postal::POSTAL_HEADERS
            .into_iter()
            .map(Cell::text),
    );
    cells.extend(
        crate::export::link::LINK_HEADERS
            .into_iter()
            .map(Cell::text),
    );
    cells
}

fn school_row(school: &CanonicalSchool) -> ReportResult<Vec<Cell>> {
    let mut cells = row!(
        Cell::text(school.id.as_str()),
        Cell::text(school.name.clone()),
        Cell::text(state_code(school)),
        optional_text(&school.city),
        optional_text(&school.association),
        optional_text(&school.classification),
        enrollment_cell(school)?,
        optional_text(&school.athletics_website),
        optional_text(&school.school_website),
        Cell::text(school.aliases.join(" | ")),
        Cell::number(school.source_identities.len())?,
        Cell::number(school.retained_conflicts.len())?,
    );
    append_school_evidence(&mut cells, school)?;
    Ok(cells)
}

fn optional_text(value: &Option<String>) -> Cell {
    Cell::text(value.clone().map_or(String::new(), core::convert::identity))
}

fn append_school_evidence(cells: &mut Vec<Cell>, school: &CanonicalSchool) -> ReportResult<()> {
    cells.extend(
        crate::export::postal::postal_fields([school])?
            .into_iter()
            .map(Cell::text),
    );
    cells.extend(
        crate::export::link::link_fields(school)
            .into_iter()
            .map(Cell::text),
    );
    Ok(())
}

fn state_code(school: &CanonicalSchool) -> &'static str {
    school
        .state
        .map_or(MEET_STATE_UNRESOLVED, UsJurisdiction::code)
}

fn enrollment_cell(school: &CanonicalSchool) -> ReportResult<Cell> {
    match school.enrollment {
        Some(enrollment) => Ok(Cell::Number(f64::from(enrollment))),
        None => Ok(Cell::Empty),
    }
}
