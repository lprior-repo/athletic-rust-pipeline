use census_domain::model::{CanonicalSchool, MEET_STATE_UNRESOLVED};
use census_domain::UsJurisdiction;

use crate::report::ReportResult;
use crate::workbook::cells::{row, Cell};

pub(super) const SCHOOL_WIDTHS: [u16; 24] = [
    16, 44, 10, 26, 14, 12, 12, 42, 42, 30, 10, 10, 20, 36, 28, 24, 12, 16, 28, 28, 28, 48, 20, 68,
];

pub(super) fn schools_sheet(schools: &[CanonicalSchool]) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = vec![row!(
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
    )];
    if let Some(header) = cells.first_mut() {
        header.extend(
            crate::export::postal::POSTAL_HEADERS
                .into_iter()
                .map(Cell::text),
        );
    }
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

fn school_row(school: &CanonicalSchool) -> ReportResult<Vec<Cell>> {
    let mut cells = row!(
        Cell::text(school.id.as_str()),
        Cell::text(school.name.clone()),
        Cell::text(state_code(school)),
        Cell::text(
            school
                .city
                .clone()
                .map_or(Default::default(), core::convert::identity)
        ),
        Cell::text(
            school
                .association
                .clone()
                .map_or(Default::default(), core::convert::identity)
        ),
        Cell::text(
            school
                .classification
                .clone()
                .map_or(Default::default(), core::convert::identity)
        ),
        enrollment_cell(school)?,
        Cell::text(
            school
                .athletics_website
                .clone()
                .map_or(Default::default(), core::convert::identity)
        ),
        Cell::text(
            school
                .school_website
                .clone()
                .map_or(Default::default(), core::convert::identity)
        ),
        Cell::text(school.aliases.join(" | ")),
        Cell::number(school.source_identities.len())?,
        Cell::number(school.retained_conflicts.len())?,
    );
    cells.extend(
        crate::export::postal::postal_fields([school])?
            .into_iter()
            .map(Cell::text),
    );
    Ok(cells)
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
