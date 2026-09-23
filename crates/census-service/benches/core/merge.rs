//! The synthetic observation batch the `merge` group folds, and the temporary store it folds from.
//!
//! The group measures the read half of the store: [`Store::scan`], which walks a table's
//! observations in key order, decodes each row, folds duplicates into one entity per canonical id
//! through [`Entity::merge`](census_store::Entity::merge), and applies the collection
//! contract to the result through `Entity::publish`. Storage is deliberately outside the measured
//! region: the batch is appended once, before any timing, and every iteration is a read.
//!
//! Seed: `Lcg::seeded(SEED)` supplies the later field values (`SEED` spells ASCII `MWCENSUS`); the
//! observations themselves come from the literal recipe below, so a run is reproducible from this
//! file. Size: `SCHOOLS * OBSERVATIONS_PER_SCHOOL` school observations and
//! `COACHES * COACH_OBSERVATIONS_PER_COACH` coach observations, each observation its own row.
//!
//! The recipe makes the merge outcome a *claim*: the first observation of a school carries the
//! enrollment and city every later observation contradicts, so a last-writer merge would show; the
//! aliases union across three observations; the third publishes a longer name that must survive
//! while the minted name does not move; and half the coaches first publish a consumer mailbox, which
//! `publish` must withhold. [`Dataset::build`] folds the batch once and refuses to hand it over
//! unless every claim holds; the row checks that refusal is built from are in `checks.rs`.

use anyhow::{ensure, Context, Result};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SourceIdentity,
    SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use tempfile::TempDir;

use super::lcg::Lcg;

#[path = "checks.rs"]
mod checks;

/// Seed of the later-value stream: ASCII `MWCENSUS`.
const SEED: u64 = 0x4D57_4345_4E53_5553;
/// The name every synthetic school is minted from, `"<SCHOOL_PREFIX> <index:04>"`.
const SCHOOL_PREFIX: &str = "Census Academy";

/// The state every synthetic school belongs to.
const STATE: UsJurisdiction = UsJurisdiction::Wisconsin;
/// Schools the batch covers.
const SCHOOLS: usize = 256;
/// Observations per school: four, so the first-writer, union and longer-name arms all run.
const OBSERVATIONS_PER_SCHOOL: usize = 4;
/// Coaches the batch covers.
const COACHES: usize = 256;
/// Observations per coach.
const OBSERVATIONS_PER_COACH: usize = 3;
/// Distinct aliases one school's observations carry between them.
const ALIASES_PER_SCHOOL: usize = 3;
/// Enrollment only the *first* observation of a school carries; later ones publish
/// `FIRST_ENROLLMENT + 1`, so first-writer-wins is visible in the folded rows.
const FIRST_ENROLLMENT: u32 = 312;
/// City only the first observation of a school carries; later ones publish a different one.
const FIRST_CITY: &str = "Bench City";
/// Names the later observations publish instead: one city per stream position.
const LATER_CITIES: [&str; 3] = ["Later City", "Second City", "Third City"];
/// Suffix the third observation appends to the school name. Longer than the name the school was
/// minted from, and prefixed by it, so the merge keeps it and leaves `normalized_name` alone.
const LONG_SUFFIX: &str = " North Campus";
/// The source and date every synthetic observation is evidence for.
const SOURCE_ID: &str = "bench";
const OBSERVED_ON: &str = "2026-09-21";

/// The temporary store, the batch sizes, and the fold that has to hold before the benches run.
pub struct Dataset {
    store: Store,
    /// Held so the temporary directory outlives the store it holds.
    _dir: TempDir,
    school_observations: usize,
    coach_observations: usize,
}

impl Dataset {
    /// Seed a temporary store with the batch and verify the fold it produces.
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

    /// Observations the school bench folds; its throughput element count.
    pub fn school_observations(&self) -> usize {
        self.school_observations
    }

    /// Observations the coach bench folds; its throughput element count.
    pub fn coach_observations(&self) -> usize {
        self.coach_observations
    }

    /// `Store::scan` over the school table: decode, merge and publish every observation.
    pub fn scan_schools(&self) -> Result<Vec<CanonicalSchool>> {
        self.store
            .scan(Table::Schools)
            .context("scanning the school table")
    }

    /// `Store::scan` over the coach table, where `publish` enforces the contact policy.
    pub fn scan_coaches(&self) -> Result<Vec<CanonicalCoach>> {
        self.store
            .scan(Table::Coaches)
            .context("scanning the coach table")
    }
}

/// `SCHOOLS` schools observed `OBSERVATIONS_PER_SCHOOL` times each.
fn school_observations() -> Vec<CanonicalSchool> {
    let mut lcg = Lcg::seeded(SEED);
    let mut batch = Vec::with_capacity(SCHOOLS.saturating_mul(OBSERVATIONS_PER_SCHOOL));
    for index in 0..SCHOOLS {
        let name = format!("{SCHOOL_PREFIX} {index:04}");
        let (school, _) = CanonicalSchool::new(STATE, &name, normalize_name(&name));
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
                    row.city = Some(lcg.pick(&LATER_CITIES, "Later City").to_string());
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

/// `COACHES` coaches observed `OBSERVATIONS_PER_COACH` times each: half first publish a school
/// mailbox, half first publish a consumer mailbox the collection contract has to withhold.
fn coach_observations() -> Vec<CanonicalCoach> {
    let mut batch = Vec::with_capacity(COACHES.saturating_mul(OBSERVATIONS_PER_COACH));
    for index in 0..COACHES {
        let name = format!("{SCHOOL_PREFIX} {index:04}");
        let (school, _) = CanonicalSchool::new(STATE, &name, normalize_name(&name));
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

/// Fold the batch and refuse it unless the merge produced exactly what the recipe claims.
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
    let (published, withheld) = checks::coach_tally(coaches)?;
    ensure!(
        published == half && withheld == half,
        "published {published}, withheld {withheld}"
    );
    Ok(())
}

/// One of the three aliases one school's observations carry between them.
fn alias(index: usize, slot: usize) -> String {
    format!("bench alias {index:04} {slot}")
}

/// A school-domain mailbox for the `index`-th coach.
fn school_mailbox(index: usize) -> String {
    format!("coach{index:04}@school.k12.wi.us")
}

/// A consumer mailbox for the `index`-th coach: one the collection contract drops.
fn consumer_mailbox(index: usize) -> String {
    format!("coach{index:04}@gmail.com")
}

/// The source every synthetic observation is evidence for.
fn source() -> SourceRef {
    SourceRef::new(SOURCE_ID, None)
}
