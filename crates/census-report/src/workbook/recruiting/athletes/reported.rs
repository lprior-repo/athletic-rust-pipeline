use super::super::dataset::Dataset;
use super::*;
use crate::workbook::reported::Projection;
use std::collections::BTreeSet;

mod summaries;

pub(super) const HEADERS: [&str; 5] = [
    "Reported Source Person IDs",
    "Reported Source Evidence",
    "Reported School ID",
    "Reported State",
    "Reported School Name",
];

pub(super) fn verified(cells: ReportResult<Vec<Cell>>, header: bool) -> ReportResult<Vec<Cell>> {
    let mut result = if header {
        row!(
            Cell::text("Record Basis"),
            Cell::text("Verification Status")
        )
    } else {
        row!(
            Cell::text("verified"),
            Cell::text("verified_identity_and_affiliation")
        )
    };
    result.extend(cells?);
    result.extend(HEADERS.map(|name| {
        if header {
            Cell::text(name)
        } else {
            Cell::Empty
        }
    }));
    Ok(result)
}

pub(super) fn row(
    dataset: &Dataset<'_>,
    athlete: &CanonicalAthlete,
    projection: &Projection<'_>,
) -> ReportResult<Vec<Cell>> {
    let mut result = row!(
        Cell::text("source_reported"),
        Cell::text("reported_unverified_needs_review"),
        Cell::text(athlete.id.as_str()),
        Cell::text(&athlete.canonical_name),
        Cell::text(athlete.gender.stable_key()),
        Cell::Number(f64::from(athlete.grad_year.get())),
        observed_school_year(athlete),
        Cell::Empty,
        Cell::Empty,
        Cell::Empty,
        Cell::Empty,
    );
    result.extend(tf_flag(athlete));
    result.extend(participation_flags(athlete));
    result.extend(summaries::cells(projection, athlete));
    let performances = projection.performances_for(&athlete.id);
    result.push(Cell::number(performances.len())?);
    result.push(Cell::number(
        performances
            .iter()
            .map(|row| &row.meet)
            .collect::<BTreeSet<_>>()
            .len(),
    )?);
    result.extend((0..19).map(|_| Cell::Empty));
    result.extend(profile_cells(profiles_of(athlete)));
    result.extend(row!(
        Cell::number(source_count(athlete))?,
        Cell::text("source_reported"),
        Cell::text("source_reported"),
        flag(!athlete.retained_conflicts.is_empty()),
        Cell::text("reported_unverified_needs_review"),
        Cell::Empty,
    ));
    result.extend(
        dataset
            .public_identities
            .into_iter()
            .flat_map(|_| [Cell::Empty, Cell::Empty]),
    );
    append_sources(&mut result, athlete, projection)?;
    Ok(result)
}

fn append_sources(
    result: &mut Vec<Cell>,
    athlete: &CanonicalAthlete,
    projection: &Projection<'_>,
) -> ReportResult<()> {
    let performances = projection.performances_for(&athlete.id);
    let identities = athlete
        .identities()
        .chain(
            performances
                .iter()
                .filter_map(|row| row.source_athlete.as_ref()),
        )
        .collect::<Vec<_>>();
    let evidence = athlete
        .evidence
        .iter()
        .chain(performances.iter().flat_map(|row| &row.evidence))
        .collect::<Vec<_>>();
    let identities = serde_json::to_string(&identities).map_err(encoding)?;
    let evidence = serde_json::to_string(&evidence).map_err(encoding)?;
    let school = projection.source.schools.get(&athlete.school);
    result.extend(row!(
        Cell::text(identities),
        Cell::text(evidence),
        Cell::text(athlete.school.as_str()),
        school
            .and_then(|school| school.state)
            .map_or(Cell::Empty, |state| Cell::text(state.code())),
        school.map_or(Cell::Empty, |school| Cell::text(&school.name)),
    ));
    Ok(())
}

fn encoding(error: serde_json::Error) -> crate::report::ReportError {
    crate::report::ReportError::Invariant {
        detail: format!("encoding reported athlete source evidence: {error}"),
    }
}
