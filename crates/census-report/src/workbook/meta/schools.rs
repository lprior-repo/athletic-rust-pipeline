use census_domain::model::{CanonicalSchool, MEET_STATE_UNRESOLVED};
use census_domain::UsJurisdiction;

use crate::report::ReportResult;
use crate::workbook::cells::{row, Cell};

pub(super) const SCHOOL_WIDTHS: [u16; 12] = [16, 44, 10, 26, 14, 12, 12, 42, 42, 30, 10, 10];

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
    Ok(row!(
        Cell::text(school.id.as_str()),
        Cell::text(school.name.clone()),
        Cell::text(state_code(school)),
        Cell::text(school.city.clone().unwrap_or_default()),
        Cell::text(school.association.clone().unwrap_or_default()),
        Cell::text(school.classification.clone().unwrap_or_default()),
        enrollment_cell(school)?,
        Cell::text(school.athletics_website.clone().unwrap_or_default()),
        Cell::text(school.school_website.clone().unwrap_or_default()),
        Cell::text(school.aliases.join(" | ")),
        Cell::number(school.source_identities.len())?,
        Cell::number(school.retained_conflicts.len())?,
    ))
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
