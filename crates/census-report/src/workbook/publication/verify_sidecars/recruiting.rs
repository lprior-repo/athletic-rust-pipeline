use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use census_domain::model::{CanonicalAthlete, CanonicalSchool};

use crate::report::ReportResult;
use crate::workbook::recruiting::contact::{self, SchoolContacts};
use crate::workbook::recruiting::profiles::profiles_of;

use super::cells::Cell;
use super::defect;
use super::input::Inputs;
use super::read::{self, Limits};

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

const LIMITS: Limits = Limits {
    bytes: 1024 * 1024 * 1024,
    records: 2_000_000,
};

pub(super) fn verify(directory: &Path, inputs: &Inputs<'_>) -> ReportResult<()> {
    let path = directory.join("recruiting.csv");
    let schools: HashMap<&str, &CanonicalSchool> = inputs
        .derivation
        .schools()
        .iter()
        .map(|school| (school.id.as_str(), school))
        .collect();
    let contacts = contact::contacts(inputs.derivation.coach_observations(), inputs.school_year);
    let postal = crate::workbook::verify::postal::athlete_index(
        inputs.dataset,
        inputs.derivation.athletes(),
    )?;
    let mut position = 0usize;
    let total = read::csv(&path, LIMITS, "recruiting.csv", |index, record| {
        if index == 1 {
            return read::headers(&path, record, &HEADERS);
        }
        let Some(athlete) = inputs.derivation.athletes().get(position) else {
            return Err(defect(format!(
                "{} record {index} is an unexpected extra athlete row",
                path.display()
            )));
        };
        let cells = cells(inputs, &schools, &contacts, &postal, athlete)?;
        read::compare_record(&path, index, record, &cells)?;
        position = position.saturating_add(1);
        Ok(())
    })?;
    if total == 0 {
        return Err(defect(format!(
            "{} is empty; the recruiting header is missing",
            path.display()
        )));
    }
    let written = total.saturating_sub(1);
    let expected = inputs.derivation.athletes().len();
    if written == expected {
        Ok(())
    } else {
        Err(defect(format!(
            "{} holds {written} athlete rows where the frozen cohort holds {expected}",
            path.display()
        )))
    }
}

fn cells(
    inputs: &Inputs<'_>,
    schools: &HashMap<&str, &CanonicalSchool>,
    contacts: &BTreeMap<String, SchoolContacts>,
    postal: &BTreeMap<String, [String; 12]>,
    athlete: &CanonicalAthlete,
) -> ReportResult<Vec<Cell>> {
    let school = schools.get(athlete.school.as_str()).copied();
    let scoped = contact::scoped(contacts.get(athlete.school.as_str()), athlete);
    let status = identity_status(inputs, athlete)?;
    let mut cells = athletic_cells(athlete, school, &scoped, status);
    let fields = postal.get(athlete.id.as_str()).ok_or_else(|| {
        defect(format!(
            "athlete {} has no frozen postal projection",
            athlete.id
        ))
    })?;
    cells.extend(fields.iter().cloned().map(Cell::text));
    Ok(cells)
}

fn identity_status(inputs: &Inputs<'_>, athlete: &CanonicalAthlete) -> ReportResult<String> {
    let status = inputs
        .dataset
        .identities()
        .status(athlete.id.as_str())
        .map_err(census_store::StoreError::from)?;
    Ok(status.as_str().to_owned())
}

fn athletic_cells(
    athlete: &CanonicalAthlete,
    school: Option<&CanonicalSchool>,
    scoped: &contact::ScopedContacts<'_>,
    status: String,
) -> Vec<Cell> {
    let profiles = profiles_of(athlete);
    let preferred = scoped.preferred();
    let sports = athlete
        .sports
        .iter()
        .map(|sport| sport.stable_key())
        .collect::<Vec<_>>()
        .join(";");
    vec![
        Cell::text(athlete.id.as_str()),
        Cell::text(athlete.canonical_name.as_str()),
        Cell::integer(athlete.grad_year.get().to_string()),
        Cell::text(athlete.gender.stable_key()),
        state_code(school),
        school_name(school),
        school_city(school),
        Cell::text(sports),
        Cell::optional(profiles.athletic_net.as_deref()),
        Cell::optional(profiles.milesplit.as_deref()),
        Cell::optional(scoped.track_names()),
        Cell::optional(scoped.track_emails()),
        Cell::optional(scoped.cross_country().map(|coach| coach.name.as_str())),
        Cell::optional(
            scoped
                .cross_country()
                .and_then(|coach| coach.address())
                .map(str::to_owned),
        ),
        Cell::optional(scoped.director().map(|coach| coach.name.as_str())),
        Cell::optional(
            scoped
                .director()
                .and_then(|coach| coach.address())
                .map(str::to_owned),
        ),
        Cell::optional(school.and_then(|school| school.athletics_website.as_deref())),
        Cell::text(preferred.source_url.as_str()),
        Cell::text(status),
        Cell::text(evidence_sources(athlete)),
    ]
}

fn state_code(school: Option<&CanonicalSchool>) -> Cell {
    Cell::text(
        school
            .and_then(|school| school.state)
            .map_or_else(String::new, |state| state.code().to_owned()),
    )
}

fn school_name(school: Option<&CanonicalSchool>) -> Cell {
    Cell::text(school.map_or_else(String::new, |school| school.name.clone()))
}

fn school_city(school: Option<&CanonicalSchool>) -> Cell {
    Cell::text(
        school
            .and_then(|school| school.city.clone())
            .unwrap_or_default(),
    )
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
