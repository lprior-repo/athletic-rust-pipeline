use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use census_domain::model::{CanonicalAthlete, CanonicalSchool, SchoolYear};

use super::contact::{contacts, scoped, ScopedContacts};
use super::profiles::profiles_of;
use crate::report::{Derivation, ReportError, ReportResult};

const HEADERS: [&str; 20] = [
    "athlete_id",
    "name",
    "grad_year",
    "gender",
    "state",
    "school",
    "school_city",
    "sports",
    "athleticnet_url",
    "milesplit_url",
    "head_track_coach",
    "head_track_coach_email",
    "head_xc_coach",
    "head_xc_coach_email",
    "athletic_director",
    "athletic_director_email",
    "athletics_website",
    "coach_source_url",
    "identity_status",
    "evidence_sources",
];

pub struct RecruitingCsvCounts {
    pub with_school_coach: usize,
    pub with_coach_email: usize,
}

pub fn write_recruiting_csv(
    derivation: &Derivation<'_>,
    school_year: SchoolYear,
    path: &Path,
) -> ReportResult<RecruitingCsvCounts> {
    let schools: HashMap<&str, &CanonicalSchool> = derivation
        .schools()
        .iter()
        .map(|school| (school.id.as_str(), school))
        .collect();
    let contact_index = contacts(derivation.coach_observations(), school_year);
    let identities = derivation.dataset().identities();
    let mut writer = csv::Writer::from_path(path).map_err(|error| csv_error(path, error))?;
    writer
        .write_record(HEADERS)
        .map_err(|error| csv_error(path, error))?;
    let mut counts = RecruitingCsvCounts {
        with_school_coach: 0,
        with_coach_email: 0,
    };
    for athlete in derivation.athletes() {
        let status = identities
            .status(athlete.id.as_str())
            .map_err(census_store::StoreError::from)?;
        let contact = scoped(contact_index.get(athlete.school.as_str()), athlete);
        let (row, has_coach, has_email) = record(
            athlete,
            schools.get(athlete.school.as_str()).copied(),
            &contact,
            status.as_str(),
        );
        writer
            .write_record(row)
            .map_err(|error| csv_error(path, error))?;
        counts.with_school_coach = increment(counts.with_school_coach, has_coach)?;
        counts.with_coach_email = increment(counts.with_coach_email, has_email)?;
    }
    writer
        .flush()
        .map_err(|source| crate::report::io_error(path, source))?;
    Ok(counts)
}

fn record(
    athlete: &CanonicalAthlete,
    school: Option<&CanonicalSchool>,
    contacts: &ScopedContacts<'_>,
    identity_status: &str,
) -> (impl Iterator<Item = String>, bool, bool) {
    let (contacts, has_coach, has_email) = contact_cells(contacts, school);
    let row = athlete_cells(athlete, school)
        .into_iter()
        .chain(contacts)
        .chain([identity_status.to_owned(), evidence_sources(athlete)]);
    (row, has_coach, has_email)
}

fn athlete_cells(athlete: &CanonicalAthlete, school: Option<&CanonicalSchool>) -> [String; 10] {
    let profiles = profiles_of(athlete);
    [
        athlete.id.to_string(),
        athlete.canonical_name.clone(),
        athlete.grad_year.get().to_string(),
        athlete.gender.stable_key().to_owned(),
        school
            .and_then(|school| school.state)
            .map_or_else(String::new, |state| state.code().to_owned()),
        school.map_or_else(String::new, |school| school.name.clone()),
        school
            .and_then(|school| school.city.clone())
            .unwrap_or_default(),
        athlete
            .sports
            .iter()
            .map(|sport| sport.stable_key())
            .collect::<Vec<_>>()
            .join(";"),
        profiles.athletic_net.unwrap_or_default(),
        profiles.milesplit.unwrap_or_default(),
    ]
}

fn contact_cells(
    contacts: &ScopedContacts<'_>,
    school: Option<&CanonicalSchool>,
) -> ([String; 8], bool, bool) {
    let preferred = contacts.preferred();
    let track_name = contacts.track_names().unwrap_or_default();
    let track_email = contacts.track_emails().unwrap_or_default();
    let xc = contacts.cross_country();
    let director = contacts.director();
    let has_coach = !track_name.is_empty() || xc.is_some();
    let has_email = !track_email.is_empty()
        || xc.is_some_and(|coach| coach.address().is_some())
        || director.is_some_and(|coach| coach.address().is_some());
    let row = [
        track_name,
        track_email,
        xc.map_or_else(String::new, |coach| coach.name.clone()),
        xc.and_then(|coach| coach.address())
            .map_or_else(String::new, str::to_owned),
        director.map_or_else(String::new, |coach| coach.name.clone()),
        director
            .and_then(|coach| coach.address())
            .map_or_else(String::new, str::to_owned),
        school
            .and_then(|school| school.athletics_website.clone())
            .unwrap_or_default(),
        preferred.source_url,
    ];
    (row, has_coach, has_email)
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

fn increment(count: usize, present: bool) -> ReportResult<usize> {
    count
        .checked_add(usize::from(present))
        .ok_or_else(|| ReportError::Invariant {
            detail: "recruiting CSV contact count exceeds usize".to_owned(),
        })
}

fn csv_error(path: &Path, source: csv::Error) -> ReportError {
    crate::report::io_error(path, std::io::Error::other(source))
}
