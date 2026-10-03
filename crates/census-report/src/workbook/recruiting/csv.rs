use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use census_domain::model::{CanonicalAthlete, CanonicalSchool, SchoolYear};

use super::contact::{contacts, scoped, ScopedContacts};
use super::profiles::profiles_of;
use crate::csv_safety::protect_owned;
use crate::report::{Derivation, ReportError, ReportResult};

const HEADERS: [&str; 32] = [
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
    "postal_school_id",
    "postal_street",
    "postal_second_line",
    "postal_city",
    "postal_state",
    "postal_zip",
    "postal_owner_namespace",
    "postal_owner_id",
    "postal_source",
    "postal_source_url",
    "postal_observed_date",
    "postal_capture_sha256",
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
    let postal =
        crate::export::postal::athlete_postal_index(derivation.dataset(), derivation.athletes())?;
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
        let fields = postal
            .get(athlete.id.as_str())
            .ok_or_else(|| ReportError::Invariant {
                detail: format!("athlete {} has no postal projection", athlete.id),
            })?;
        write_row(&mut writer, row.chain(fields.iter().cloned()), path)?;
        counts.with_school_coach = increment(counts.with_school_coach, has_coach)?;
        counts.with_coach_email = increment(counts.with_coach_email, has_email)?;
    }
    writer
        .flush()
        .map_err(|source| crate::report::io_error(path, source))?;
    Ok(counts)
}

fn write_row<W: std::io::Write>(
    writer: &mut csv::Writer<W>,
    row: impl Iterator<Item = String>,
    path: &Path,
) -> ReportResult<()> {
    for field in row {
        let field = protect_owned(field).map_err(|source| literal_error(path, source))?;
        writer
            .write_field(field.as_str())
            .map_err(|error| csv_error(path, error))?;
    }
    writer
        .write_record(std::iter::empty::<&str>())
        .map_err(|error| csv_error(path, error))
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
            .map_or(Default::default(), core::convert::identity),
        athlete
            .sports
            .iter()
            .map(|sport| sport.stable_key())
            .collect::<Vec<_>>()
            .join(";"),
        profiles
            .athletic_net
            .map_or(Default::default(), core::convert::identity),
        profiles
            .milesplit
            .map_or(Default::default(), core::convert::identity),
    ]
}

fn contact_cells(
    contacts: &ScopedContacts<'_>,
    school: Option<&CanonicalSchool>,
) -> ([String; 8], bool, bool) {
    let preferred = contacts.preferred();
    let track_name = contacts
        .track_names()
        .map_or(Default::default(), core::convert::identity);
    let track_email = contacts
        .track_emails()
        .map_or(Default::default(), core::convert::identity);
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
            .map_or(Default::default(), core::convert::identity),
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

fn literal_error(path: &Path, source: std::collections::TryReserveError) -> ReportError {
    ReportError::Invariant {
        detail: format!(
            "allocating literal CSV text for {}: {source}",
            path.display()
        ),
    }
}

fn csv_error(path: &Path, source: csv::Error) -> ReportError {
    crate::report::io_error(path, std::io::Error::other(source))
}

#[cfg(test)]
mod tests;
