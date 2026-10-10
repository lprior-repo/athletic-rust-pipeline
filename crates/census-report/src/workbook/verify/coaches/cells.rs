use super::super::canonical::Value;
use super::super::expectations::Expectations;
use crate::workbook::recruiting::coach_projection;
use census_domain::model::{CanonicalCoach, CoachRole, SchoolYear};

pub(super) fn values(expectations: &Expectations<'_>, coach: &CanonicalCoach) -> Vec<Value> {
    let school = coach.school.as_str();
    let contacts = expectations.contacts.get(school);
    let director = contacts.and_then(|facts| facts.director());
    let selected = coach_projection::admitted(coach, expectations.school_year, contacts);
    let [_, name_digest, _] = coach_projection::capture(selected.name);
    let mut values = identity_values(expectations, coach);
    values.extend(contact_values(
        coach,
        expectations.school_year,
        &selected,
        director,
    ));
    values.extend(capture_values(
        selected.professional.map(|contact| contact.tenure()),
    ));
    values.extend(capture_values(
        selected.personal.map(|contact| contact.tenure()),
    ));
    values.push(Value::text(name_digest));
    values.extend(director_values(director));
    values.push(Value::text(selected.state));
    values
}

fn contact_values(
    coach: &CanonicalCoach,
    year: SchoolYear,
    selected: &coach_projection::Projection<'_>,
    director: Option<&crate::workbook::recruiting::contact::Named>,
) -> [Value; 9] {
    let [name_url, _, name_at] = coach_projection::capture(selected.name);
    [
        Value::optional(selected.professional.map(|contact| contact.mailbox())),
        Value::optional(selected.personal.map(|contact| contact.mailbox())),
        Value::optional(coach.phone.as_deref()),
        Value::optional(director.map(|row| row.name.as_str())),
        Value::optional(director.and_then(|row| row.email.as_deref())),
        Value::text(name_url),
        Value::text(name_at),
        Value::text(coach_projection::tenure_label(coach, year)),
        Value::text(year.short()),
    ]
}

fn identity_values(expectations: &Expectations<'_>, coach: &CanonicalCoach) -> Vec<Value> {
    let school = coach.school.as_str();
    vec![
        Value::text(school),
        Value::text(expectations.school_name(school)),
        Value::text(expectations.school_city(school)),
        Value::text(expectations.school_state(school)),
        Value::text(sport_label(coach)),
        Value::text(&coach.name),
        Value::text(coach.id.as_str()),
        Value::text(coach.gender.stable_key()),
        Value::text(coach.role.stable_key()),
    ]
}

fn capture_values(fact: Option<&census_domain::model::CoachTenureEvidence>) -> [Value; 3] {
    coach_projection::capture(fact).map(Value::text)
}

fn director_values(director: Option<&crate::workbook::recruiting::contact::Named>) -> [Value; 3] {
    let source = director.and_then(|row| row.professional_source());
    [
        Value::optional(source.map(|source| source.source_url.as_str())),
        Value::optional(source.map(|source| source.source_sha256.as_str())),
        Value::optional(source.map(|source| source.observed_on.as_str())),
    ]
}

pub(super) fn sport_label(coach: &CanonicalCoach) -> String {
    match (coach.sport, coach.role) {
        (Some(sport), _) => sport.stable_key().to_owned(),
        (None, CoachRole::AthleticDirector) => "school_wide".to_owned(),
        (None, _) => "unknown".to_owned(),
    }
}
