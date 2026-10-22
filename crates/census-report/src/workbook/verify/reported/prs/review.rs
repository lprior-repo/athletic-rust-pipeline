use super::*;
use crate::workbook::reported::ReportedReview;
use census_domain::model::performance_comparison::{Measure, SurfaceClass};

pub(super) fn values(
    projection: &Projection<'_>,
    review: &ReportedReview<'_>,
    expectations: &Expectations<'_>,
) -> ReportResult<Vec<Value>> {
    let performance = review.performance;
    let mut values = vec![
        Value::text("source_reported"),
        Value::text("reported_event_context_needs_review_not_a_best"),
    ];
    values.extend(identity_values(projection, review));
    values.extend(result_values(projection, review));
    values.extend(context_values(projection, review));
    values.extend(
        public_identity::sheet_headers(expectations.dataset, &[])
            .into_iter()
            .map(|_| Value::Empty),
    );
    values.extend([Value::Empty, Value::Empty, Value::Empty]);
    values.push(Value::optional(
        performance
            .source_athlete
            .as_ref()
            .map(|source| source.id.as_str()),
    ));
    values.push(json(&performance.evidence)?);
    values.push(json(review.interpretation)?);
    Ok(values)
}

fn identity_values(projection: &Projection<'_>, review: &ReportedReview<'_>) -> [Value; 9] {
    let athlete = projection.athlete(&review.performance.athlete);
    let meet = projection.meet(&review.performance.meet);
    [
        Value::text(review.performance.athlete.as_str()),
        Value::optional(athlete.map(|row| row.canonical_name.as_str())),
        Value::optional(athlete.map(|row| row.gender.stable_key())),
        Value::Empty,
        Value::Empty,
        athlete.map_or(Value::Empty, |row| {
            Value::integer(i64::from(row.grad_year.get()))
        }),
        Value::text(crate::bests::sport_of(review.interpretation.kind())),
        Value::text(review.interpretation.kind().stable_key()),
        Value::optional(meet.map(|row| SurfaceClass::resolve(row).season_label())),
    ]
}

fn result_values(projection: &Projection<'_>, review: &ReportedReview<'_>) -> [Value; 10] {
    let row = review.performance;
    [
        Value::Empty,
        Measure::of(&row.mark)
            .and_then(|measure| measure.normalized_mark(&row.mark))
            .map_or(Value::Empty, Value::decimal),
        Value::optional(mark_unit(&row.mark)),
        row.wind_mps.map_or(Value::Empty, Value::decimal),
        Value::text(&row.date),
        Value::optional(projection.meet(&row.meet).map(|meet| meet.name.as_str())),
        row.place
            .map_or(Value::Empty, |place| Value::integer(i64::from(place))),
        Value::optional(
            row.evidence
                .iter()
                .filter_map(|item| item.source.url.as_deref())
                .find(|url| !url.is_empty()),
        ),
        Value::Empty,
        Value::text("comparison and performance-time affiliation unresolved"),
    ]
}

fn context_values(projection: &Projection<'_>, review: &ReportedReview<'_>) -> [Value; 10] {
    let row = review.performance;
    [
        Value::optional(
            projection
                .meet(&row.meet)
                .map(|meet| SurfaceClass::resolve(meet).label()),
        ),
        Value::text("unknown"),
        Value::text(row.timing.map_or("unknown", |value| value.stable_key())),
        Value::text(row.id.as_str()),
        Value::text(row.meet.as_str()),
        Value::text(&row.source_key),
        Value::text(review.interpretation.basis()),
        Value::text(crate::bests::mark_text(&row.mark)),
        Value::Empty,
        Value::Empty,
    ]
}

fn json(value: &impl serde::Serialize) -> ReportResult<Value> {
    serde_json::to_string(value)
        .map(Value::text)
        .map_err(|_| super::super::invalid("encoding expected reported event interpretation"))
}
