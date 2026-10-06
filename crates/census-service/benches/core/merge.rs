use anyhow::{ensure, Context, Result};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SourceIdentity,
    SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use tempfile::TempDir;

#[path = "checks.rs"]
mod checks;

const SCHOOL_PREFIX: &str = "Census Academy";

const STATE: UsJurisdiction = UsJurisdiction::Wisconsin;
const SCHOOLS: usize = 256;
const OBSERVATIONS_PER_SCHOOL: usize = 4;
const COACHES: usize = 256;
const OBSERVATIONS_PER_COACH: usize = 3;
const ALIASES_PER_SCHOOL: usize = 3;
const FIRST_ENROLLMENT: u32 = 312;
const FIRST_CITY: &str = "Bench City";
const LONG_SUFFIX: &str = " North Campus";
const SOURCE_ID: &str = "bench";
const OBSERVED_ON: &str = "2026-09-21";

pub struct Dataset {
    store: Store,
    _dir: TempDir,
    school_observations: usize,
    coach_observations: usize,
}

impl Dataset {
    pub fn build() -> Result<Self> {
        let dir = tempfile::tempdir().context("creating the temporary store directory")?;
        let store = Store::open(dir.path()).context("opening the temporary census store")?;
        let schools = school_observations();
        let coaches = coach_observations();
        store
            .append_many(Table::Schools, &schools)
            .context("appending the school batch")?;
        store
            .append_many(Table::Coaches, &coaches)
            .context("appending the coach batch")?;
        let dataset = Self {
            store,
            _dir: dir,
            school_observations: schools.len(),
            coach_observations: coaches.len(),
        };
        verify(&dataset.scan_schools()?, &dataset.scan_coaches()?)?;
        Ok(dataset)
    }

    pub fn school_observations(&self) -> usize {
        self.school_observations
    }

    pub fn coach_observations(&self) -> usize {
        self.coach_observations
    }

    pub fn scan_schools(&self) -> Result<Vec<CanonicalSchool>> {
        self.store
            .scan(Table::Schools)
            .context("scanning the school table")
    }

    pub fn scan_coaches(&self) -> Result<Vec<CanonicalCoach>> {
        self.store
            .scan(Table::Coaches)
            .context("scanning the coach table")
    }
}

fn school_observations() -> Vec<CanonicalSchool> {
    let mut batch = Vec::with_capacity(SCHOOLS.saturating_mul(OBSERVATIONS_PER_SCHOOL));
    for index in 0..SCHOOLS {
        let name = format!("{SCHOOL_PREFIX} {index:04}");
        let (school, _) = CanonicalSchool::new(STATE, &name, normalize_name(&name), None);
        for observation in 0..OBSERVATIONS_PER_SCHOOL {
            let mut row = school.clone();
            match observation {
                0 => {
                    row.city = Some(FIRST_CITY.to_string());
                    row.enrollment = Some(FIRST_ENROLLMENT);
                    row.aliases = vec![alias(index, 0), alias(index, 1)];
                    row.source_identities.push(SourceIdentity::new(
                        SourceNamespace::AssociationSchool {
                            association: "wiaa".to_string(),
                        },
                        format!("bench-{index:04}"),
                    ));
                }
                1 => {
                    row.city = Some(FIRST_CITY.to_string());
                    row.enrollment = Some(FIRST_ENROLLMENT.saturating_add(1));
                    row.association = Some("WIAA".to_string());
                    row.co_op = true;
                    row.aliases = vec![alias(index, 1), alias(index, 2)];
                    row.evidence.push(Evidence::parsed(source(), OBSERVED_ON));
                }
                2 => {
                    row.name = format!("{name}{LONG_SUFFIX}");
                    row.classification = Some("Division 2".to_string());
                    row.athletics_website =
                        Some(format!("https://athletics.example.org/{index:04}"));
                }
                _ => {
                    row.aliases = vec![alias(index, 2)];
                    row.evidence.push(Evidence::fetched(source(), OBSERVED_ON));
                }
            }
            batch.push(row);
        }
    }
    batch
}
fn coach_observations() -> Vec<CanonicalCoach> {
    let mut batch = Vec::with_capacity(COACHES.saturating_mul(OBSERVATIONS_PER_COACH));
    for index in 0..COACHES {
        let name = format!("{SCHOOL_PREFIX} {index:04}");
        let (school, _) = CanonicalSchool::new(STATE, &name, normalize_name(&name), None);
        let coach = CanonicalCoach::new(
            &school.id,
            format!("Bench Coach {index:04}"),
            Some(Sport::OutdoorTrack),
            Gender::Boys,
            CoachRole::HeadCoach,
        );
        for observation in 0..OBSERVATIONS_PER_COACH {
            let mut row = coach.clone();
            match observation {
                0 if index % 2 == 0 => row.professional_email = Some(school_mailbox(index)),
                0 => row.professional_email = Some(consumer_mailbox(index)),
                1 => row.professional_email = Some(consumer_mailbox(index)),
                _ => {
                    row.phone = Some(format!("+1-555-{index:04}"));
                    row.evidence.push(Evidence::fetched(source(), OBSERVED_ON));
                }
            }
            batch.push(row);
        }
    }
    batch
}

fn verify(schools: &[CanonicalSchool], coaches: &[CanonicalCoach]) -> Result<()> {
    ensure!(
        schools.len() == SCHOOLS,
        "folded {} school rows, not {SCHOOLS}",
        schools.len()
    );
    ensure!(
        coaches.len() == COACHES,
        "folded {} coach rows, not {COACHES}",
        coaches.len()
    );
    for row in schools {
        checks::school_row(row)?;
    }
    let half = COACHES / 2;
    let (professional, personal) = checks::coach_tally(coaches)?;
    ensure!(
        professional == half && personal == COACHES,
        "professional {professional}, personal {personal}; every seeded address must survive"
    );
    Ok(())
}

fn alias(index: usize, slot: usize) -> String {
    format!("bench alias {index:04} {slot}")
}

fn school_mailbox(index: usize) -> String {
    format!("coach{index:04}@school.k12.wi.us")
}
fn consumer_mailbox(index: usize) -> String {
    format!("coach{index:04}@gmail.com")
}

fn source() -> SourceRef {
    SourceRef::new(SOURCE_ID, None)
}
