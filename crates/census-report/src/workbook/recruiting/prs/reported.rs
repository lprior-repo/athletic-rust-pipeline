use super::*;
use crate::workbook::reported::{Projection, ReportedBest};
use census_domain::model::performance_comparison::mark_unit;
use std::collections::BTreeSet;

pub(super) const HEADERS: [&str; 2] = ["Source Athlete ID", "Performance Evidence"];

pub(super) fn verified(cells: ReportResult<Vec<Cell>>, header: bool) -> ReportResult<Vec<Cell>> {
    let mut result = if header {
        vec![
            Cell::text("Record Basis"),
            Cell::text("Verification Status"),
        ]
    } else {
        vec![Cell::text("verified"), Cell::text("verified_eligible_best")]
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
    reported: &ReportedBest<'_>,
    projection: &Projection<'_>,
    identity: bool,
    schools: bool,
) -> ReportResult<Vec<Cell>> {
    let mut result = vec![
        Cell::text("source_reported"),
        Cell::text(if reported.performance.is_some() {
            "reported_best_in_retained_set"
        } else {
            "reported_no_comparable_candidate_needs_review"
        }),
    ];
    result.extend(identity_cells(reported, projection));
    result.extend(result_cells(reported, projection)?);
    result.extend(context_cells(reported));
    result.extend(
        identity
            .then_some([Cell::Empty, Cell::Empty])
            .into_iter()
            .flatten(),
    );
    result.extend(
        schools
            .then_some([Cell::Empty, Cell::Empty, Cell::Empty])
            .into_iter()
            .flatten(),
    );
    result.push(
        reported
            .performance
            .and_then(|row| row.source_athlete.as_ref())
            .map_or(Cell::Empty, |source| Cell::text(&source.id)),
    );
    result.push(
        reported
            .performance
            .map(|row| serde_json::to_string(&row.evidence))
            .transpose()
            .map_err(|error| ReportError::Invariant {
                detail: format!("encoding reported performance evidence: {error}"),
            })?
            .map_or(Cell::Empty, Cell::text),
    );
    Ok(result)
}

fn identity_cells(reported: &ReportedBest<'_>, projection: &Projection<'_>) -> [Cell; 9] {
    let key = &reported.group.key;
    let athlete = reported.athlete;
    let school = reported
        .performance
        .and_then(|row| projection.source.teams.get(&row.team))
        .and_then(|team| projection.source.schools.get(&team.school));
    [
        Cell::text(key.athlete_id.as_str()),
        athlete.map_or(Cell::Empty, |athlete| Cell::text(&athlete.canonical_name)),
        athlete.map_or(Cell::Empty, |athlete| {
            Cell::text(athlete.gender.stable_key())
        }),
        school.map_or(Cell::Empty, |school| Cell::text(&school.name)),
        school
            .and_then(|school| school.state)
            .map_or(Cell::Empty, |state| Cell::text(state.code())),
        athlete.map_or(Cell::Empty, |athlete| {
            Cell::Number(f64::from(athlete.grad_year.get()))
        }),
        Cell::text(crate::bests::sport_of(&key.event_kind)),
        Cell::text(key.event_kind.stable_key()),
        Cell::text(key.surface.season_label()),
    ]
}

fn result_cells(
    reported: &ReportedBest<'_>,
    projection: &Projection<'_>,
) -> ReportResult<[Cell; 10]> {
    let performance = reported.performance;
    let url = performance.and_then(|row| {
        row.evidence
            .iter()
            .filter_map(|evidence| evidence.source.url.as_deref())
            .find(|url| !url.is_empty())
    });
    let sources = performance.map(|row| {
        row.evidence
            .iter()
            .map(|evidence| &evidence.source.id)
            .collect::<BTreeSet<_>>()
            .len()
    });
    Ok([
        performance.map_or(Cell::Empty, |row| {
            Cell::text(crate::bests::mark_text(&row.mark))
        }),
        performance
            .and_then(|row| reported.group.key.measure.normalized_mark(&row.mark))
            .map_or(Cell::Empty, Cell::Number),
        performance
            .and_then(|row| mark_unit(&row.mark))
            .map_or(Cell::Empty, Cell::text),
        performance
            .and_then(|row| row.wind_mps)
            .map_or(Cell::Empty, Cell::Number),
        performance.map_or(Cell::Empty, |row| Cell::text(&row.date)),
        performance
            .and_then(|row| projection.meet(&row.meet))
            .map_or(Cell::Empty, |meet| Cell::text(&meet.name)),
        performance
            .and_then(|row| row.place)
            .map_or(Cell::Empty, |place| Cell::Number(f64::from(place))),
        url.map_or(Cell::Empty, Cell::text),
        sources
            .map(Cell::number)
            .transpose()?
            .map_or(Cell::Empty, core::convert::identity),
        super::conflict_cell(&reported.group.conflicts),
    ])
}

fn context_cells(reported: &ReportedBest<'_>) -> [Cell; 10] {
    let key = &reported.group.key;
    let performance = reported.performance;
    let adjustment = performance.and_then(|row| row.adjustment.as_ref());
    [
        Cell::text(key.surface.label()),
        Cell::text(key.wind_class.label()),
        Cell::text(key.timing.label()),
        performance.map_or(Cell::Empty, |row| Cell::text(row.id.as_str())),
        performance.map_or(Cell::Empty, |row| Cell::text(row.meet.as_str())),
        performance.map_or(Cell::Empty, |row| Cell::text(&row.source_key)),
        key.context
            .as_ref()
            .map_or(Cell::Empty, |id| Cell::text(id.as_str())),
        adjustment
            .and_then(|value| value.original_mark.as_ref())
            .map_or(Cell::Empty, |mark| {
                Cell::text(crate::bests::mark_text(mark))
            }),
        adjustment.map_or(Cell::Empty, |value| Cell::text(value.basis.label())),
        adjustment
            .and_then(|value| value.note.as_deref())
            .map_or(Cell::Empty, Cell::text),
    ]
}
