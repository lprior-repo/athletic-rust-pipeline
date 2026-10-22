use super::super::athletes::cells::{observed_season, profiles_of, source_count};
use super::super::athletes::{expected_ids, ordered_athletes, values};
use super::super::canonical::Value;
use super::super::expectations::Expectations;
use super::super::labels::{self, athlete_headers::Schema};
use super::super::public_identity;
use super::super::read::Book;
use super::super::report::Findings;
use crate::report::ReportResult;
use crate::workbook::reported::Projection;
use census_domain::model::{CanonicalAthlete, CanonicalPerformance, Sport};
use std::collections::BTreeSet;

mod summaries;

pub(super) fn verify(
    book: &mut Book,
    expectations: &Expectations<'_>,
    projection: &Projection<'_>,
    findings: &mut Findings,
) -> ReportResult<()> {
    let schema = Schema::of(expectations.dataset.semantic_policy()?);
    let verified = ordered_athletes(expectations);
    expected_ids(expectations, &verified)?;
    let headers = headers(expectations, schema);
    let total = verified
        .len()
        .checked_add(projection.athletes.len())
        .ok_or_else(|| super::invalid("mixed athlete count overflow"))?;
    super::rows::verify(
        book,
        labels::ATHLETES,
        &headers,
        total,
        |position, findings| {
            if let Some(athlete) = verified.get(position) {
                let mut row = vec![
                    Value::text("verified"),
                    Value::text("verified_identity_and_affiliation"),
                ];
                row.extend(values(expectations, schema, athlete, findings)?);
                row.extend([
                    Value::Empty,
                    Value::Empty,
                    Value::Empty,
                    Value::Empty,
                    Value::Empty,
                ]);
                return Ok(row);
            }
            let position = position
                .checked_sub(verified.len())
                .ok_or_else(|| super::invalid("mixed athlete position underflow"))?;
            let athlete = projection.athletes.get(position).ok_or_else(|| {
                super::invalid("reported athlete row is absent from frozen projection")
            })?;
            reported_values(projection, athlete, schema, expectations)
        },
        findings,
    )
}

fn reported_values(
    projection: &Projection<'_>,
    athlete: &CanonicalAthlete,
    schema: Schema,
    expectations: &Expectations<'_>,
) -> ReportResult<Vec<Value>> {
    let performances = projection.performances_for(&athlete.id);
    let profiles = profiles_of(athlete);
    let mut row = vec![
        Value::text("source_reported"),
        Value::text("reported_unverified_needs_review"),
        Value::text(athlete.id.as_str()),
        Value::text(&athlete.canonical_name),
        Value::text(athlete.gender.stable_key()),
        Value::integer(i64::from(athlete.grad_year.get())),
        Value::optional(observed_season(athlete).map(|year| year.short())),
        Value::Empty,
        Value::Empty,
        Value::Empty,
        Value::Empty,
        Value::flagged(
            athlete
                .sports
                .iter()
                .any(|sport| matches!(sport, Sport::IndoorTrack | Sport::OutdoorTrack)),
        ),
        Value::flagged(athlete.sports.contains(&Sport::CrossCountry)),
        Value::flagged(athlete.sports.contains(&Sport::IndoorTrack)),
        Value::flagged(athlete.sports.contains(&Sport::OutdoorTrack)),
    ];
    row.extend(summaries::values(projection, athlete, schema));
    row.push(Value::count(performances.len()));
    row.push(Value::count(
        performances
            .iter()
            .map(|row| &row.meet)
            .collect::<BTreeSet<_>>()
            .len(),
    ));
    row.extend((0..19).map(|_| Value::Empty));
    row.extend([
        Value::optional(profiles.athletic_net),
        Value::optional(profiles.milesplit),
        Value::text(profiles.other.join("; ")),
        Value::count(source_count(athlete)),
        Value::text("source_reported"),
        Value::text("source_reported"),
        Value::flagged(!athlete.retained_conflicts.is_empty()),
        Value::text("reported_unverified_needs_review"),
        Value::Empty,
    ]);
    row.extend(
        public_identity::sheet_headers(expectations.dataset, &[])
            .into_iter()
            .map(|_| Value::Empty),
    );
    row.extend(source_values(projection, athlete, performances)?);
    Ok(row)
}

fn expected_evidence(value: &str) -> Value {
    use sha2::{Digest, Sha256};

    let units = value.encode_utf16().count();
    match units {
        0..=32_767 => Value::text(value),
        _ => {
            let digest = Sha256::digest(value.as_bytes());
            Value::text(format!(
                "Excel display overflow: {units} UTF-16 units; SHA-256 {digest:x}; full evidence in frozen-input.json; performance evidence in reported_performances.csv"
            ))
        }
    }
}

fn headers(expectations: &Expectations<'_>, schema: Schema) -> Vec<&'static str> {
    let mut headers = vec!["Record Basis", "Verification Status"];
    headers.extend(
        public_identity::sheet_headers(expectations.dataset, schema.headers())
            .into_iter()
            .map(|header| match header {
                "Conflict Flag" => "Retained Source Conflict",
                "Review Status" => "Identity Authority",
                other => other,
            }),
    );
    headers.extend([
        "Reported Source Person IDs",
        "Reported Source Evidence",
        "Reported School ID",
        "Reported State",
        "Reported School Name",
    ]);
    headers
}

fn source_values(
    projection: &Projection<'_>,
    athlete: &CanonicalAthlete,
    performances: &[&CanonicalPerformance],
) -> ReportResult<[Value; 5]> {
    let source_ids = athlete
        .identities()
        .chain(
            performances
                .iter()
                .filter_map(|row| row.source_athlete.as_ref()),
        )
        .collect::<Vec<_>>();
    let source_evidence = athlete
        .evidence
        .iter()
        .chain(performances.iter().flat_map(|row| row.evidence.iter()))
        .collect::<Vec<_>>();
    let source_ids = serde_json::to_string(&source_ids)
        .map_err(|_| super::invalid("encoding expected reported source identities"))?;
    let evidence = serde_json::to_string(&source_evidence)
        .map_err(|_| super::invalid("encoding expected reported athlete evidence"))?;
    let school = projection.source.schools.get(&athlete.school);
    Ok([
        Value::text(source_ids),
        expected_evidence(&evidence),
        Value::text(athlete.school.as_str()),
        Value::optional(
            school
                .and_then(|school| school.state)
                .map(|state| state.code()),
        ),
        Value::optional(school.map(|school| school.name.as_str())),
    ])
}
