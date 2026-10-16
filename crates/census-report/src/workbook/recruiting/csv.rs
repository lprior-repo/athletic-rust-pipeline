use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use census_domain::model::{CanonicalAthlete, CanonicalSchool, SchoolYear};

use super::bounded::{bounded_contacts, WORKBOOK_GROUP_BUDGET};
use super::contact::{attach_research, scoped, SchoolContacts};
use crate::csv_safety::protect_owned;
use crate::report::{Derivation, ReportError, ReportResult};
use crate::workbook::public_identity::{self, Identity};

mod rows;

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

pub struct RecruitingCsvCounts {
    pub with_school_coach: usize,
    pub with_coach_email: usize,
}

pub fn write_recruiting_csv(
    derivation: &Derivation<'_>,
    school_year: SchoolYear,
    path: &Path,
) -> ReportResult<RecruitingCsvCounts> {
    derivation.dataset().ensure_current_selected_write()?;
    derivation.ensure_selected_affiliation()?;
    public_identity::preflight(derivation.dataset(), derivation.athletes())?;
    crate::report::EventAdmission::of(derivation.dataset())?;
    let projection = CsvProjection::new(derivation, school_year)?;
    let mut writer = open_writer(path, projection.public_identities)?;
    let counts = write_athletes(derivation, &projection, &mut writer, path)?;
    writer
        .flush()
        .map_err(|source| crate::report::io_error(path, source))?;
    Ok(counts)
}

fn open_writer(
    path: &Path,
    identities: Option<&crate::export::public_identity::VerifiedPublicIdentityView>,
) -> ReportResult<csv::Writer<std::fs::File>> {
    let mut writer = csv::Writer::from_path(path).map_err(|error| csv_error(path, error))?;
    writer
        .write_record(public_identity::headers(&HEADERS, identities))
        .map_err(|error| csv_error(path, error))?;
    Ok(writer)
}

fn write_athletes(
    derivation: &Derivation<'_>,
    projection: &CsvProjection<'_>,
    writer: &mut csv::Writer<std::fs::File>,
    path: &Path,
) -> ReportResult<RecruitingCsvCounts> {
    let mut counts = RecruitingCsvCounts {
        with_school_coach: 0,
        with_coach_email: 0,
    };
    for athlete in derivation.athletes() {
        let (row, has_coach, has_email) =
            projection.identified_record(athlete, derivation.dataset())?;
        write_row(writer, row, path)?;
        counts.with_school_coach = increment(counts.with_school_coach, has_coach)?;
        counts.with_coach_email = increment(counts.with_coach_email, has_email)?;
    }
    Ok(counts)
}

fn researched_contacts(
    derivation: &Derivation<'_>,
    school_year: SchoolYear,
) -> ReportResult<BTreeMap<String, SchoolContacts>> {
    let mut contacts = bounded_contacts(
        derivation.coach_observations(),
        school_year,
        WORKBOOK_GROUP_BUDGET,
    )?;
    attach_research(&mut contacts, derivation.schools(), school_year);
    Ok(contacts)
}

struct CsvProjection<'a> {
    public_identities: Option<&'a crate::export::public_identity::VerifiedPublicIdentityView>,
    schools: HashMap<&'a str, &'a CanonicalSchool>,
    contacts: BTreeMap<String, SchoolContacts>,
    school_address: BTreeMap<String, String>,
}

impl<'a> CsvProjection<'a> {
    fn new(derivation: &'a Derivation<'_>, school_year: SchoolYear) -> ReportResult<Self> {
        crate::grouping::Admission::new("recruiting schools").rows(derivation.schools())?;
        Ok(Self {
            public_identities: public_identity::view(derivation.dataset())?,
            schools: derivation
                .schools()
                .iter()
                .map(|school| (school.id.as_str(), school))
                .collect(),
            contacts: researched_contacts(derivation, school_year)?,
            school_address: crate::export::postal::athlete_address_index(
                derivation.dataset(),
                derivation.athletes(),
            )?,
        })
    }

    fn identified_record(
        &self,
        athlete: &CanonicalAthlete,
        dataset: &crate::export::ExportDataset,
    ) -> ReportResult<(impl Iterator<Item = String> + use<>, bool, bool)> {
        let status = match self.public_identities {
            Some(_) => "accepted_public_identity",
            None => dataset
                .identities()
                .status(athlete.id.as_str())
                .map_err(census_store::StoreError::from)?
                .as_str(),
        };
        self.record(athlete, status)
    }

    fn record(
        &self,
        athlete: &CanonicalAthlete,
        status: &str,
    ) -> ReportResult<(impl Iterator<Item = String> + use<>, bool, bool)> {
        let identity = Identity::of(self.public_identities, athlete.id.as_str())?;
        let contact = scoped(self.contacts.get(athlete.school.as_str()), athlete);
        let (row, has_coach, has_email) = rows::record(
            athlete,
            self.schools.get(athlete.school.as_str()).copied(),
            &contact,
            status,
            identity.athlete_id(),
        )?;
        let address = self
            .school_address
            .get(athlete.id.as_str())
            .ok_or_else(|| ReportError::Invariant {
                detail: format!("athlete {} has no school address projection", athlete.id),
            })?;
        Ok((
            row.chain([address.clone()]).chain(identity.audit()?),
            has_coach,
            has_email,
        ))
    }
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
