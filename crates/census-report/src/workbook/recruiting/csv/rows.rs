use super::super::contact::ScopedContacts;
use super::super::profiles::profiles_of;
use crate::report::ReportResult;
use census_domain::model::{CanonicalAthlete, CanonicalSchool};
use std::collections::BTreeSet;

pub(super) fn record(
    athlete: &CanonicalAthlete,
    school: Option<&CanonicalSchool>,
    contacts: &ScopedContacts<'_>,
    identity_status: &str,
) -> ReportResult<(impl Iterator<Item = String> + use<>, bool, bool)> {
    let (contacts, has_coach, has_email) = contact_cells(contacts, school)?;
    let row = athlete_cells(athlete, school)
        .into_iter()
        .chain(contacts)
        .chain([identity_status.to_owned(), evidence_sources(athlete)]);
    Ok((row, has_coach, has_email))
}

fn athlete_cells(athlete: &CanonicalAthlete, school: Option<&CanonicalSchool>) -> [String; 10] {
    let profiles = profiles_of(athlete);
    let [state, school_name, city] = school_cells(school);
    [
        athlete.id.to_string(),
        athlete.canonical_name.clone(),
        athlete.grad_year.get().to_string(),
        athlete.gender.stable_key().to_owned(),
        state,
        school_name,
        city,
        sports(athlete),
        profiles
            .athletic_net
            .map_or(Default::default(), core::convert::identity),
        profiles
            .milesplit
            .map_or(Default::default(), core::convert::identity),
    ]
}

fn school_cells(school: Option<&CanonicalSchool>) -> [String; 3] {
    [
        school
            .and_then(|school| school.state)
            .map_or_else(String::new, |state| state.code().to_owned()),
        school.map_or_else(String::new, |school| school.name.clone()),
        school
            .and_then(|school| school.city.clone())
            .map_or(Default::default(), core::convert::identity),
    ]
}

fn sports(athlete: &CanonicalAthlete) -> String {
    athlete
        .sports
        .iter()
        .map(|sport| sport.stable_key())
        .collect::<Vec<_>>()
        .join(";")
}

fn contact_cells(
    contacts: &ScopedContacts<'_>,
    school: Option<&CanonicalSchool>,
) -> ReportResult<(impl Iterator<Item = String> + use<>, bool, bool)> {
    let roles = role_cells(contacts, school);
    let (has_coach, has_email) = contact_flags(&roles);
    let capture = capture_cells(contacts)?;
    Ok((roles.into_iter().chain(capture), has_coach, has_email))
}

fn contact_flags(roles: &[String; 7]) -> (bool, bool) {
    let [track_name, track_email, xc_name, xc_email, _, ad_email, _] = roles;
    (
        !track_name.is_empty() || !xc_name.is_empty(),
        !track_email.is_empty() || !xc_email.is_empty() || !ad_email.is_empty(),
    )
}

fn capture_cells(contacts: &ScopedContacts<'_>) -> ReportResult<[String; 5]> {
    let preferred = contacts.preferred();
    Ok([
        preferred.source_url.to_owned(),
        preferred.source_sha256.to_owned(),
        preferred.observed_on.to_owned(),
        preferred.coach_id.to_owned(),
        super::super::mailbox_provenance::json(contacts)?,
    ])
}

fn role_cells(contacts: &ScopedContacts<'_>, school: Option<&CanonicalSchool>) -> [String; 7] {
    let xc = contacts.cross_country();
    let director = contacts.director();
    [
        contacts
            .track_names()
            .map_or(String::new(), core::convert::identity),
        contacts
            .track_emails()
            .map_or(String::new(), core::convert::identity),
        xc.map_or_else(String::new, |coach| coach.name.clone()),
        optional_text(xc.and_then(|coach| coach.address())),
        director.map_or_else(String::new, |coach| coach.name.clone()),
        optional_text(director.and_then(|coach| coach.address())),
        optional_text(school.and_then(|school| school.athletics_website.as_deref())),
    ]
}

fn optional_text(value: Option<&str>) -> String {
    value.map_or_else(String::new, str::to_owned)
}

fn evidence_sources(athlete: &CanonicalAthlete) -> String {
    athlete
        .evidence
        .iter()
        .map(|evidence| evidence.source.id.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(";")
}
