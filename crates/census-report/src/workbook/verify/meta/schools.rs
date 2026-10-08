use census_domain::model::{CanonicalSchool, MEET_STATE_UNRESOLVED};
use census_domain::UsJurisdiction;

use crate::report::ReportResult;

use super::{header, Expect, Sheet};

const HEADERS: [&str; 33] = [
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
    "Postal School ID",
    "Postal Street",
    "Postal Second Line",
    "Postal City",
    "Postal State",
    "Postal ZIP",
    "Postal Owner Namespace",
    "Postal Owner ID",
    "Postal Source",
    "Postal Source URL",
    "Postal Observed Date",
    "Postal Capture SHA256",
    "Postal Address Kind",
    "Link School ID",
    "Link Owner Namespace",
    "Link Owner ID",
    "Link Owner URL",
    "Link Source",
    "Link Source URL",
    "Link Observed Date",
    "Link Note",
];

pub(super) fn expected(schools: &[CanonicalSchool]) -> ReportResult<Sheet> {
    let mut rows = vec![header(&HEADERS)];
    let mut sorted: Vec<&CanonicalSchool> = schools.iter().collect();
    sorted.sort_by(|left, right| {
        state_code(left)
            .cmp(state_code(right))
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.as_str().cmp(right.id.as_str()))
    });
    for school in sorted {
        rows.push(row(school)?);
    }
    Ok(("Schools", rows))
}

fn row(school: &CanonicalSchool) -> ReportResult<Vec<Expect>> {
    let mut cells = vec![
        Expect::text(school.id.as_str()),
        Expect::text(school.name.as_str()),
        Expect::text(state_code(school)),
        optional_text(school.city.as_deref()),
        optional_text(school.association.as_deref()),
        optional_text(school.classification.as_deref()),
        enrollment(school),
        optional_text(school.athletics_website.as_deref()),
        optional_text(school.school_website.as_deref()),
        Expect::text(school.aliases.join(" | ")),
        Expect::count(school.source_identities.len())?,
        Expect::count(school.retained_conflicts.len())?,
    ];
    append_evidence(&mut cells, school)?;
    Ok(cells)
}

fn append_evidence(cells: &mut Vec<Expect>, school: &CanonicalSchool) -> ReportResult<()> {
    cells.extend(
        crate::workbook::verify::postal::fields([school])?
            .into_iter()
            .map(Expect::text),
    );
    cells.extend(
        crate::workbook::verify::link::fields(school)
            .into_iter()
            .map(Expect::text),
    );
    Ok(())
}

fn optional_text(value: Option<&str>) -> Expect {
    Expect::text(value.map_or("", core::convert::identity))
}

fn enrollment(school: &CanonicalSchool) -> Expect {
    match school.enrollment {
        Some(value) => Expect::Number(f64::from(value)),
        None => Expect::Empty,
    }
}

fn state_code(school: &CanonicalSchool) -> &'static str {
    school
        .state
        .map_or(MEET_STATE_UNRESOLVED, UsJurisdiction::code)
}
