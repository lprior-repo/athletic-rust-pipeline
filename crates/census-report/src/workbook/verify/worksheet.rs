use super::{athletes, current_school::AffiliationExpectations, expectations::Expectations};
use super::{
    canonical::Value, postal, public_identity::Expected, reach::Reach, read::Book, report::Findings,
};
use crate::report::{ReportError, ReportResult};
use crate::workbook::reported::Projection;
use census_domain::model::CanonicalAthlete;
use std::collections::BTreeSet;

mod marks;

const HEADERS: [&str; 24] = [
    "Athlete ID",
    "Name",
    "Gender",
    "Graduation Year",
    "Observed Grade",
    "Grade School Year",
    "School",
    "School ID",
    "School City",
    "School State",
    "School Address",
    "School Basis",
    "Current school",
    "Current state",
    "Current school as-of",
    "Head TF Coach",
    "Head XC Coach",
    "Observed bests",
    "Reported bests",
    "Unresolved results",
    "Source-declared PRs",
    "Source Profile",
    "Record Basis",
    "Verification Status",
];

#[derive(Clone, Copy)]
enum Subject<'a> {
    Accepted(&'a CanonicalAthlete),
    Reported(&'a CanonicalAthlete),
}

pub(super) fn verify(
    book: &mut Book,
    expected: &Expectations<'_>,
    findings: &mut Findings,
) -> ReportResult<()> {
    let accepted = athletes::ordered_athletes(expected);
    athletes::expected_ids(expected, &accepted)?;
    let reported = Projection::of(
        expected.dataset,
        &expected.derivation,
        &expected.admission,
        &expected.bests,
    )?;
    let total = accepted
        .len()
        .checked_add(reported.as_ref().map_or(0, |rows| rows.athletes.len()))
        .ok_or_else(|| invalid("single worksheet population overflow"))?;
    super::reported::rows::verify(
        book,
        "Athletes",
        &HEADERS,
        total,
        |position, _| {
            let subject = match accepted.get(position) {
                Some(athlete) => Subject::Accepted(athlete),
                None => {
                    let index = position
                        .checked_sub(accepted.len())
                        .ok_or_else(|| invalid("worksheet position underflow"))?;
                    let athlete = reported
                        .as_ref()
                        .and_then(|rows| rows.athletes.get(index))
                        .ok_or_else(|| {
                            invalid("worksheet subject absent from frozen population")
                        })?;
                    Subject::Reported(athlete)
                }
            };
            values(expected, subject, reported.as_ref())
        },
        findings,
    )
}

fn values(
    expected: &Expectations<'_>,
    subject: Subject<'_>,
    reported: Option<&Projection<'_>>,
) -> ReportResult<Vec<Value>> {
    let athlete = match subject {
        Subject::Accepted(athlete) | Subject::Reported(athlete) => athlete,
    };
    let id = match subject {
        Subject::Accepted(_) => Expected::of(&expected.identities, athlete.id.as_str())?.athlete_id,
        Subject::Reported(_) => athlete.id.as_str().to_owned(),
    };
    let mut row = vec![
        Value::text(id),
        Value::text(&athlete.canonical_name),
        Value::text(athlete.gender.stable_key()),
        Value::integer(i64::from(athlete.grad_year.get())),
    ];
    row.extend(grade(athlete));
    row.extend(school(expected, athlete, subject)?);
    row.extend(current(expected, athlete, subject)?);
    row.extend(coaches(expected, athlete, subject));
    row.extend(marks::values(expected, athlete, reported)?);
    row.push(Value::Empty);
    row.push(Value::optional(
        athletes::cells::profiles_of(athlete).milesplit,
    ));
    row.extend(match subject {
        Subject::Accepted(_) => [
            Value::text("verified"),
            Value::text("verified_identity_and_affiliation"),
        ],
        Subject::Reported(_) => [
            Value::text("source_reported"),
            Value::text("reported_unverified_needs_review"),
        ],
    });
    Ok(row)
}

fn grade(athlete: &CanonicalAthlete) -> [Value; 2] {
    let Some(last) = athlete
        .observed_grades
        .iter()
        .max_by_key(|grade| grade.school_year.get())
    else {
        return [Value::Empty, Value::Empty];
    };
    let grades = athlete
        .observed_grades
        .iter()
        .filter(|grade| grade.school_year == last.school_year)
        .map(|grade| grade.grade.get())
        .collect::<BTreeSet<_>>();
    [
        Value::text(
            grades
                .into_iter()
                .map(|grade| grade.to_string())
                .collect::<Vec<_>>()
                .join(" | "),
        ),
        Value::text(last.school_year.short()),
    ]
}

fn school(
    expected: &Expectations<'_>,
    athlete: &CanonicalAthlete,
    subject: Subject<'_>,
) -> ReportResult<[Value; 6]> {
    let school = expected.dataset.schools.get(&athlete.school);
    let mut addresses =
        postal::athlete_address_index(expected.dataset, std::slice::from_ref(athlete))?;
    let address = addresses
        .remove(athlete.id.as_str())
        .ok_or_else(|| invalid("frozen worksheet postal projection absent"))?;
    Ok([
        Value::optional(school.map(|school| school.name.as_str())),
        Value::text(athlete.school.as_str()),
        Value::optional(school.and_then(|school| school.city.as_deref())),
        Value::optional(
            school
                .and_then(|school| school.state)
                .map(|state| state.code()),
        ),
        Value::text(address),
        Value::text(match subject {
            Subject::Accepted(_) => "accepted_current_affiliation",
            Subject::Reported(_) => "source_reported_affiliation",
        }),
    ])
}

fn current(
    expected: &Expectations<'_>,
    athlete: &CanonicalAthlete,
    subject: Subject<'_>,
) -> ReportResult<[Value; 3]> {
    match subject {
        Subject::Reported(_) => Ok([Value::text("Unknown"), Value::text("Unknown"), Value::Empty]),
        Subject::Accepted(_) => {
            let [_, name, state, as_of] =
                AffiliationExpectations::of(expected.dataset)?.values(athlete.id.as_str())?;
            Ok([Value::text(name), Value::text(state), Value::text(as_of)])
        }
    }
}

fn coaches(
    expected: &Expectations<'_>,
    athlete: &CanonicalAthlete,
    subject: Subject<'_>,
) -> [Value; 2] {
    match subject {
        Subject::Reported(_) => [Value::Empty, Value::Empty],
        Subject::Accepted(_) => {
            let reach = Reach::of(expected, athlete);
            [reach.track_names, reach.cross_country_name]
        }
    }
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.to_owned(),
    }
}
