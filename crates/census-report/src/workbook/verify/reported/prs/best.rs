use super::*;
use crate::workbook::reported::ReportedBest;
use std::collections::BTreeSet;

pub(super) fn values(
    projection: &Projection<'_>,
    reported: &ReportedBest<'_>,
    expectations: &Expectations<'_>,
) -> ReportResult<Vec<Value>> {
    let performance = reported.performance;
    let evidence = performance
        .map(|performance| serde_json::to_string(&performance.evidence))
        .transpose()
        .map_err(|_| super::super::invalid("encoding expected reported performance evidence"))?;
    let mut row = identity_values(projection, reported);
    row.extend(result_values(projection, reported));
    row.extend(source_values(reported));
    row.push(Value::text(
        reported
            .group
            .conflicts
            .iter()
            .map(|conflict| format!("{}: {}", conflict.meet, conflict.marks.join(" | ")))
            .collect::<Vec<_>>()
            .join("; "),
    ));
    row.extend(context_values(reported));
    row.extend(
        public_identity::sheet_headers(expectations.dataset, &[])
            .into_iter()
            .map(|_| Value::Empty),
    );
    row.extend([Value::Empty, Value::Empty, Value::Empty]);
    row.extend([
        Value::optional(
            performance
                .and_then(|performance| performance.source_athlete.as_ref())
                .map(|identity| identity.id.as_str()),
        ),
        Value::optional(evidence),
        Value::Empty,
    ]);
    Ok(row)
}

fn identity_values(projection: &Projection<'_>, reported: &ReportedBest<'_>) -> Vec<Value> {
    let athlete = projection.athlete(&reported.group.key.athlete_id);
    let school = reported
        .performance
        .and_then(|performance| projection.source.teams.get(&performance.team))
        .and_then(|team| projection.source.schools.get(&team.school));
    vec![
        Value::text("source_reported"),
        Value::text(if reported.performance.is_some() {
            "reported_best_in_retained_set"
        } else {
            "reported_no_comparable_candidate_needs_review"
        }),
        Value::text(reported.group.key.athlete_id.as_str()),
        Value::optional(athlete.map(|athlete| athlete.canonical_name.as_str())),
        Value::optional(athlete.map(|athlete| athlete.gender.stable_key())),
        Value::optional(school.map(|school| school.name.as_str())),
        Value::optional(
            school
                .and_then(|school| school.state)
                .map(|state| state.code()),
        ),
        athlete.map_or(Value::Empty, |athlete| {
            Value::integer(i64::from(athlete.grad_year.get()))
        }),
    ]
}

fn result_values(projection: &Projection<'_>, reported: &ReportedBest<'_>) -> [Value; 10] {
    let key = &reported.group.key;
    let performance = reported.performance;
    let meet = performance.and_then(|performance| projection.meet(&performance.meet));
    [
        Value::text(crate::bests::sport_of(&key.event_kind)),
        Value::text(key.event_kind.stable_key()),
        Value::text(key.surface.season_label()),
        performance.map_or(Value::Empty, |performance| {
            Value::text(crate::bests::mark_text(&performance.mark))
        }),
        performance
            .and_then(|performance| key.measure.normalized_mark(&performance.mark))
            .map_or(Value::Empty, Value::decimal),
        Value::optional(performance.and_then(|performance| mark_unit(&performance.mark))),
        performance
            .and_then(|performance| performance.wind_mps)
            .map_or(Value::Empty, Value::decimal),
        Value::optional(performance.map(|performance| performance.date.as_str())),
        Value::optional(meet.map(|meet| meet.name.as_str())),
        performance
            .and_then(|performance| performance.place)
            .map_or(Value::Empty, |place| Value::integer(i64::from(place))),
    ]
}

fn source_values(reported: &ReportedBest<'_>) -> [Value; 2] {
    let performance = reported.performance;
    let url = performance.and_then(|performance| {
        performance
            .evidence
            .iter()
            .filter_map(|evidence| evidence.source.url.as_deref())
            .find(|url| !url.is_empty())
    });
    let source_count = performance.map(|performance| {
        performance
            .evidence
            .iter()
            .map(|evidence| &evidence.source.id)
            .collect::<BTreeSet<_>>()
            .len()
    });
    [
        Value::optional(url),
        source_count.map_or(Value::Empty, Value::count),
    ]
}

fn context_values(reported: &ReportedBest<'_>) -> [Value; 10] {
    let key = &reported.group.key;
    let performance = reported.performance;
    let adjustment = performance.and_then(|performance| performance.adjustment.as_ref());
    [
        Value::text(key.surface.label()),
        Value::text(key.wind_class.label()),
        Value::text(key.timing.label()),
        Value::optional(performance.map(|performance| performance.id.as_str())),
        Value::optional(performance.map(|performance| performance.meet.as_str())),
        Value::optional(performance.map(|performance| performance.source_key.as_str())),
        Value::optional(key.context.as_ref().map(|context| context.as_str())),
        adjustment
            .and_then(|adjustment| adjustment.original_mark.as_ref())
            .map_or(Value::Empty, |mark| {
                Value::text(crate::bests::mark_text(mark))
            }),
        Value::optional(adjustment.map(|adjustment| adjustment.basis.label())),
        Value::optional(adjustment.and_then(|adjustment| adjustment.note.as_deref())),
    ]
}
