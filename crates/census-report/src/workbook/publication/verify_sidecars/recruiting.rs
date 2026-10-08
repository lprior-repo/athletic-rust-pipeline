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

const HEADERS: [&str; 25] = [
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
    "coach_capture_sha256",
    "coach_acquired_at",
    "coach_id",
    "contact_provenance",
    "identity_status",
    "evidence_sources",
    crate::export::postal::ATHLETE_ADDRESS_CSV_HEADER,
];

const LIMITS: Limits = Limits {
    bytes: 1024 * 1024 * 1024,
    records: 2_000_000,
};

pub(super) fn verify(directory: &Path, inputs: &Inputs<'_>) -> ReportResult<()> {
    let path = directory.join("recruiting.csv");
    let projection = Projection::of(inputs)?;
    let mut position = 0usize;
    let total = read::csv(&path, LIMITS, "recruiting.csv", |index, record| {
        projection.row(&path, index, record, &mut position)
    })?;
    verify_count(&path, total, inputs.derivation.athletes().len())
}

struct Projection<'a, 'd> {
    inputs: &'a Inputs<'d>,
    schools: HashMap<&'a str, &'a CanonicalSchool>,
    contacts: BTreeMap<String, SchoolContacts>,
    school_address: BTreeMap<String, String>,
}

impl<'a, 'd> Projection<'a, 'd> {
    fn of(inputs: &'a Inputs<'d>) -> ReportResult<Self> {
        let schools = inputs
            .derivation
            .schools()
            .iter()
            .map(|school| (school.id.as_str(), school))
            .collect();
        let contacts =
            contact::contacts(inputs.derivation.coach_observations(), inputs.school_year);
        let school_address = crate::workbook::verify::postal::athlete_address_index(
            inputs.dataset,
            inputs.derivation.athletes(),
        )?;
        Ok(Self {
            inputs,
            schools,
            contacts,
            school_address,
        })
    }

    fn row(
        &self,
        path: &Path,
        index: usize,
        record: &csv::StringRecord,
        position: &mut usize,
    ) -> ReportResult<()> {
        if index == 1 {
            return read::headers(path, record, &HEADERS);
        }
        let athlete = self.athlete(path, index, *position)?;
        let expected = cells(
            self.inputs,
            &self.schools,
            &self.contacts,
            &self.school_address,
            athlete,
        )?;
        read::compare_record(path, index, record, &expected)?;
        *position = position.saturating_add(1);
        Ok(())
    }

    fn athlete(
        &self,
        path: &Path,
        index: usize,
        position: usize,
    ) -> ReportResult<&CanonicalAthlete> {
        self.inputs
            .derivation
            .athletes()
            .get(position)
            .ok_or_else(|| {
                defect(format!(
                    "{} record {index} is an unexpected extra athlete row",
                    path.display()
                ))
            })
    }
}

fn verify_count(path: &Path, total: usize, expected: usize) -> ReportResult<()> {
    if total == 0 {
        return Err(defect(format!(
            "{} is empty; the recruiting header is missing",
            path.display()
        )));
    }
    let written = total.saturating_sub(1);
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
    school_address: &BTreeMap<String, String>,
    athlete: &CanonicalAthlete,
) -> ReportResult<Vec<Cell>> {
    let school = schools.get(athlete.school.as_str()).copied();
    let scoped = contact::scoped(contacts.get(athlete.school.as_str()), athlete);
    let status = identity_status(inputs, athlete)?;
    let mut cells = athletic_cells(athlete, school, &scoped, status)?;
    let address = school_address.get(athlete.id.as_str()).ok_or_else(|| {
        defect(format!(
            "athlete {} has no frozen school address projection",
            athlete.id
        ))
    })?;
    cells.push(Cell::optional(
        (!address.is_empty()).then_some(address.as_str()),
    ));
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
) -> ReportResult<Vec<Cell>> {
    let preferred = scoped.preferred();
    let mut cells = athlete_cells(athlete, school);
    cells.extend(role_cells(scoped, school));
    cells.extend([
        Cell::text(preferred.source_url),
        Cell::text(preferred.source_sha256),
        Cell::text(preferred.observed_on),
        Cell::text(preferred.coach_id),
        Cell::text(crate::workbook::recruiting::mailbox_provenance::json(
            scoped,
        )?),
        Cell::text(status),
        Cell::text(evidence_sources(athlete)),
    ]);
    Ok(cells)
}

fn athlete_cells(athlete: &CanonicalAthlete, school: Option<&CanonicalSchool>) -> Vec<Cell> {
    let profiles = profiles_of(athlete);
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
    ]
}

fn role_cells(scoped: &contact::ScopedContacts<'_>, school: Option<&CanonicalSchool>) -> [Cell; 7] {
    [
        Cell::optional(scoped.track_names()),
        Cell::optional(scoped.track_emails()),
        Cell::optional(scoped.cross_country().map(|coach| coach.name.as_str())),
        Cell::optional(scoped.cross_country().and_then(|coach| coach.address())),
        Cell::optional(scoped.director().map(|coach| coach.name.as_str())),
        Cell::optional(scoped.director().and_then(|coach| coach.address())),
        Cell::optional(school.and_then(|school| school.athletics_website.as_deref())),
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
            .map_or(Default::default(), core::convert::identity),
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
