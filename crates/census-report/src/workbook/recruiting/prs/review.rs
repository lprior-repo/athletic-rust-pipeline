use super::*;
use crate::workbook::reported::{Projection, ReportedReview};
use census_domain::model::performance_comparison::{mark_unit, Measure, SurfaceClass};

pub(in crate::workbook::recruiting) fn row(
    review: &ReportedReview<'_>,
    projection: &Projection<'_>,
    identity: bool,
    schools: bool,
) -> ReportResult<Vec<Cell>> {
    let performance = review.performance;
    let mut result = vec![
        Cell::text("source_reported"),
        Cell::text("reported_event_context_needs_review_not_a_best"),
    ];
    result.extend(identity_cells(review, projection));
    result.extend(result_cells(review, projection));
    result.extend(context_cells(review, projection));
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
        performance
            .source_athlete
            .as_ref()
            .map_or(Cell::Empty, |source| Cell::text(&source.id)),
    );
    result.push(json(&performance.evidence)?);
    result.push(json(review.interpretation)?);
    Ok(result)
}

fn identity_cells(review: &ReportedReview<'_>, projection: &Projection<'_>) -> [Cell; 9] {
    let athlete = projection.athlete(&review.performance.athlete);
    let meet = projection.meet(&review.performance.meet);
    [
        Cell::text(review.performance.athlete.as_str()),
        athlete.map_or(Cell::Empty, |row| Cell::text(&row.canonical_name)),
        athlete.map_or(Cell::Empty, |row| Cell::text(row.gender.stable_key())),
        Cell::Empty,
        Cell::Empty,
        athlete.map_or(Cell::Empty, |row| {
            Cell::Number(f64::from(row.grad_year.get()))
        }),
        Cell::text(crate::bests::sport_of(review.interpretation.kind())),
        Cell::text(review.interpretation.kind().stable_key()),
        meet.map_or(Cell::Empty, |row| {
            Cell::text(SurfaceClass::resolve(row).season_label())
        }),
    ]
}

fn result_cells(review: &ReportedReview<'_>, projection: &Projection<'_>) -> [Cell; 10] {
    let row = review.performance;
    [
        Cell::Empty,
        Measure::of(&row.mark)
            .and_then(|measure| measure.normalized_mark(&row.mark))
            .map_or(Cell::Empty, Cell::Number),
        mark_unit(&row.mark).map_or(Cell::Empty, Cell::text),
        row.wind_mps.map_or(Cell::Empty, Cell::Number),
        Cell::text(&row.date),
        projection
            .meet(&row.meet)
            .map_or(Cell::Empty, |meet| Cell::text(&meet.name)),
        row.place
            .map_or(Cell::Empty, |place| Cell::Number(f64::from(place))),
        row.evidence
            .iter()
            .filter_map(|item| item.source.url.as_deref())
            .find(|url| !url.is_empty())
            .map_or(Cell::Empty, Cell::text),
        Cell::Empty,
        Cell::text("comparison and performance-time affiliation unresolved"),
    ]
}

fn context_cells(review: &ReportedReview<'_>, projection: &Projection<'_>) -> [Cell; 10] {
    let row = review.performance;
    [
        projection.meet(&row.meet).map_or(Cell::Empty, |meet| {
            Cell::text(SurfaceClass::resolve(meet).label())
        }),
        Cell::text("unknown"),
        row.timing.map_or_else(
            || Cell::text("unknown"),
            |value| Cell::text(value.stable_key()),
        ),
        Cell::text(row.id.as_str()),
        Cell::text(row.meet.as_str()),
        Cell::text(&row.source_key),
        Cell::text(review.interpretation.basis()),
        Cell::text(crate::bests::mark_text(&row.mark)),
        Cell::Empty,
        Cell::Empty,
    ]
}

fn json(value: &impl serde::Serialize) -> ReportResult<Cell> {
    serde_json::to_string(value)
        .map(Cell::text)
        .map_err(|error| ReportError::Invariant {
            detail: format!("encoding reported event review evidence: {error}"),
        })
}
