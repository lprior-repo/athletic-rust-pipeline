use super::super::canonical::Value;
use super::super::current_school::AffiliationExpectations;
use super::super::expectations::Expectations;
use super::super::labels;
use super::super::public_identity;
use super::super::read::Book;
use super::super::report::Findings;
use crate::report::ReportResult;
use crate::workbook::reported::{Projection, ReportedBest};
use census_domain::model::performance_comparison::mark_unit;
use std::collections::BTreeSet;

pub(super) fn verify(
    book: &mut Book,
    expectations: &Expectations<'_>,
    projection: &Projection<'_>,
    findings: &mut Findings,
) -> ReportResult<()> {
    let schools = AffiliationExpectations::of(expectations.dataset)?;
    let mut headers = vec!["Record Basis", "Verification Status"];
    headers.extend(
        public_identity::sheet_headers(expectations.dataset, &labels::PR_HEADERS)
            .into_iter()
            .map(|header| match header {
                "School" => "School at performance",
                "State" => "State at performance",
                other => other,
            }),
    );
    headers.extend(["Current school", "Current state", "Current school as-of"]);
    headers.extend(["Source Athlete ID", "Performance Evidence"]);
    let total = expectations
        .bests
        .len()
        .checked_add(projection.bests.len())
        .ok_or_else(|| super::invalid("mixed PR count overflow"))?;
    super::rows::verify(
        book,
        labels::PRS,
        &headers,
        total,
        |position, _| {
            if let Some(pr) = expectations.bests.get(position) {
                let mut row = vec![
                    Value::text("verified"),
                    Value::text("verified_eligible_best"),
                ];
                row.extend(super::super::prs::values(pr, expectations, Some(&schools))?);
                row.extend([Value::Empty, Value::Empty]);
                return Ok(row);
            }
            let position = position
                .checked_sub(expectations.bests.len())
                .ok_or_else(|| super::invalid("mixed PR position underflow"))?;
            let reported = projection.bests.get(position).ok_or_else(|| {
                super::invalid("reported PR row is absent from frozen projection")
            })?;
            reported_values(projection, reported, expectations)
        },
        findings,
    )
}

fn reported_values(
    projection: &Projection<'_>,
    reported: &ReportedBest<'_>,
    expectations: &Expectations<'_>,
) -> ReportResult<Vec<Value>> {
    let key = &reported.group.key;
    let performance = reported.performance;
    let athlete = projection.athlete(&key.athlete_id);
    let school = performance
        .and_then(|performance| projection.source.teams.get(&performance.team))
        .and_then(|team| projection.source.schools.get(&team.school));
    let meet = performance.and_then(|performance| projection.meet(&performance.meet));
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
    let conflicts = reported
        .group
        .conflicts
        .iter()
        .map(|conflict| format!("{}: {}", conflict.meet, conflict.marks.join(" | ")))
        .collect::<Vec<_>>()
        .join("; ");
    let adjustment = performance.and_then(|performance| performance.adjustment.as_ref());
    let evidence = performance
        .map(|performance| serde_json::to_string(&performance.evidence))
        .transpose()
        .map_err(|_| super::invalid("encoding expected reported performance evidence"))?;
    let mut row = vec![
        Value::text("source_reported"),
        Value::text(if performance.is_some() {
            "reported_best_in_retained_set"
        } else {
            "reported_no_comparable_candidate_needs_review"
        }),
        Value::text(key.athlete_id.as_str()),
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
        Value::optional(url),
        source_count.map_or(Value::Empty, Value::count),
        Value::text(conflicts),
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
    ];
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
    ]);
    Ok(row)
}
