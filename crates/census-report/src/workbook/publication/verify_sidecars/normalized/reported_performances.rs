use super::rows;
use crate::report::{ReportError, ReportResult};
use crate::workbook::reported::Projection;
use census_domain::model::CanonicalPerformance;
use std::path::Path;

pub(super) fn verify(directory: &Path, projection: &Projection<'_>) -> ReportResult<u64> {
    let mut expected = projection.performances.iter();
    rows::read_table(
        &directory.join("reported_performances.csv"),
        crate::workbook::publication::normalized::reported_performances::HEADER,
        projection.performances.len(),
        |row| {
            let performance = expected.next().ok_or_else(|| ReportError::Invariant {
                detail: "reported performance row exceeds frozen projection".into(),
            })?;
            verify_row(row, projection, performance)
        },
    )
}

fn verify_row(
    row: &rows::Row<'_>,
    projection: &Projection<'_>,
    performance: &CanonicalPerformance,
) -> ReportResult<()> {
    verify_identity(row, projection, performance)?;
    verify_result(row, projection, performance)
}

fn verify_identity(
    row: &rows::Row<'_>,
    projection: &Projection<'_>,
    performance: &CanonicalPerformance,
) -> ReportResult<()> {
    let athlete =
        projection
            .athlete(&performance.athlete)
            .ok_or_else(|| ReportError::Invariant {
                detail: "frozen reported performance has no exact athlete parent".into(),
            })?;
    let team = projection.source.teams.get(&performance.team);
    let school = team.and_then(|team| projection.source.schools.get(&team.school));
    row.literals(&[
        ("Record Basis", "source_reported"),
        ("Verification Status", "reported_unverified_needs_review"),
        ("Canonical Athlete ID", performance.athlete.as_str()),
        ("Name (canonical record)", &athlete.canonical_name),
        (
            "School at performance",
            school.map_or("", |school| school.name.as_str()),
        ),
        (
            "State at performance",
            school
                .and_then(|school| school.state)
                .map_or("", |state| state.code()),
        ),
    ])?;
    row.json(
        "Source Person IDs",
        &athlete
            .identities()
            .chain(performance.source_athlete.iter())
            .collect::<Vec<_>>(),
    )?;
    row.number(
        "Graduation Year (canonical record)",
        athlete.grad_year.get(),
    )?;
    row.json("Source Team IDs", &team.map(|team| &team.source_identities))
}

fn verify_result(
    row: &rows::Row<'_>,
    projection: &Projection<'_>,
    performance: &CanonicalPerformance,
) -> ReportResult<()> {
    let event = projection.event(&performance.event);
    let meet = projection.meet(&performance.meet);
    row.literals(&[
        ("Performance ID", performance.id.as_str()),
        ("Event ID", performance.event.as_str()),
        ("Meet ID", performance.meet.as_str()),
        ("Meet", meet.map_or("", |meet| meet.name.as_str())),
        ("Date", &performance.date),
        ("Source Key", &performance.source_key),
    ])?;
    row.text(
        "Event",
        event.map_or_else(String::new, |event| event.kind.stable_key().into_owned()),
    )?;
    row.text("Mark", crate::bests::mark_text(&performance.mark))?;
    row.text(
        "Timing",
        performance.timing.map_or("", |timing| timing.stable_key()),
    )?;
    row.text(
        "Wind",
        performance
            .wind_mps
            .map_or_else(String::new, |wind| wind.to_string()),
    )?;
    row.text(
        "Place",
        performance
            .place
            .map_or_else(String::new, |place| place.to_string()),
    )?;
    row.json("Performance Evidence", &performance.evidence)?;
    row.json("Retained Conflicts", &performance.retained_conflicts)?;
    match projection.interpretation(performance)? {
        Some(interpretation) => row.json("Reported Event Interpretation", interpretation),
        None => row.text("Reported Event Interpretation", ""),
    }
}
