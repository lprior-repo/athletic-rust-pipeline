use crate::report::ReportResult;
use census_domain::model::{
    AppliedAthleteIdentity, AthleteIdentityIndex, AthleteIdentityProjection, CanonicalAthlete,
    CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, CensusRun, CoachLaneObligation, CoverageRow, GradYear, RelayResult, ReviewCase,
    ReviewVerdictRecord, SchoolId, SourceAccessCondition, TeamId,
};
use census_store::{Store, Table};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

mod frozen;
mod read;

pub use frozen::{CancelOutcome, FrozenRoot};

pub(crate) const MAX_FROZEN_INPUT_BYTES: u64 = 200 * 1024 * 1024 * 1024;

static RESIDENT_GENERATIONS: AtomicU64 = AtomicU64::new(0);
static PEAK_RESIDENT_GENERATIONS: AtomicU64 = AtomicU64::new(0);
static DECODED_GENERATIONS: AtomicU64 = AtomicU64::new(0);

pub fn resident_generations() -> u64 {
    RESIDENT_GENERATIONS.load(Ordering::Relaxed)
}

pub fn peak_resident_generations() -> u64 {
    PEAK_RESIDENT_GENERATIONS.load(Ordering::Relaxed)
}

pub fn decoded_generations() -> u64 {
    DECODED_GENERATIONS.load(Ordering::Relaxed)
}

fn note_generation() {
    let resident = RESIDENT_GENERATIONS
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);
    PEAK_RESIDENT_GENERATIONS.fetch_max(resident, Ordering::Relaxed);
    DECODED_GENERATIONS.fetch_add(1, Ordering::Relaxed);
}

fn release_generation() {
    RESIDENT_GENERATIONS.fetch_sub(1, Ordering::Relaxed);
}

#[derive(Serialize)]
pub struct ExportDataset {
    pub athletes: Vec<CanonicalAthlete>,
    pub schools: BTreeMap<SchoolId, CanonicalSchool>,
    pub teams: BTreeMap<TeamId, CanonicalTeam>,
    pub coaches: Vec<CanonicalCoach>,
    pub coach_observations: Vec<CanonicalCoach>,
    pub events: Vec<CanonicalEvent>,
    pub meets: Vec<CanonicalMeet>,
    pub performances: Vec<CanonicalPerformance>,
    pub review_cases: Vec<ReviewCase>,
    pub source_access: Vec<SourceAccessCondition>,
    pub verdicts: Vec<ReviewVerdictRecord>,
    pub identity_decisions: Vec<AppliedAthleteIdentity>,
    #[serde(skip)]
    pub canonical_aliases: HashMap<String, String>,
    #[serde(skip)]
    pub lineage: DatasetLineage,
    pub coverage_evidence: CoverageEvidence,
    pub relays: Vec<RelayResult>,
    #[serde(skip)]
    identity_projection: Arc<AthleteIdentityProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageEvidence {
    pub rows: Vec<CoverageRow>,
    pub coach_lanes: Vec<CoachLaneObligation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetLineage {
    pub store_root: String,
    pub generated_on: String,
    pub store_identity: String,
    pub export_job: Option<String>,
    pub input_generation: String,
    pub input_digest: String,
    pub source_digest: String,
    pub snapshot_sequence: u64,
    pub schema_revision: u32,
    pub policy_revision: u32,
    #[serde(default)]
    pub run: Option<CensusRun>,
    #[serde(default)]
    pub cohort: Option<GradYear>,
}

pub fn store_identity(store: &Store) -> ReportResult<String> {
    frozen::store_identity(store)
}

pub fn store_lineage(store: &Store) -> ReportResult<DatasetLineage> {
    store.store_identity()?;
    let snapshot = store.snapshot();
    frozen::lineage(store, &snapshot)
}

pub fn freeze_v2(store: &Store, job: &str) -> ReportResult<FrozenRoot> {
    frozen::freeze_v2(store, job)
}

pub fn cancel_freeze_v2(store: &Store, job: &str) -> ReportResult<CancelOutcome> {
    frozen::cancel_freeze_v2(store, job)
}

pub fn to_export_dataset(root: &FrozenRoot, store: &Store) -> ReportResult<ExportDataset> {
    frozen::to_export_dataset(root, store)
}

#[derive(Deserialize)]
struct Loaded {
    athletes: Vec<CanonicalAthlete>,
    #[serde(deserialize_with = "frozen::map_values")]
    schools: Vec<CanonicalSchool>,
    #[serde(deserialize_with = "frozen::map_values")]
    teams: Vec<CanonicalTeam>,
    coaches: Vec<CanonicalCoach>,
    coach_observations: Vec<CanonicalCoach>,
    events: Vec<CanonicalEvent>,
    meets: Vec<CanonicalMeet>,
    performances: Vec<CanonicalPerformance>,
    review_cases: Vec<ReviewCase>,
    source_access: Vec<SourceAccessCondition>,
    verdicts: Vec<ReviewVerdictRecord>,
    identity_decisions: Vec<AppliedAthleteIdentity>,
    coverage_evidence: CoverageEvidence,
    relays: Vec<RelayResult>,
}

fn projection_of(loaded: &Loaded) -> ReportResult<Arc<AthleteIdentityProjection>> {
    let mut index = AthleteIdentityIndex::default();
    for athlete in &loaded.athletes {
        index
            .observe(athlete)
            .map_err(census_store::StoreError::from)?;
    }
    Ok(Arc::new(census_store::build_athlete_identity_projection(
        index,
        &loaded.review_cases,
        &loaded.verdicts,
        &loaded.identity_decisions,
    )?))
}

fn aliases_of(
    athletes: &[CanonicalAthlete],
    projection: &AthleteIdentityProjection,
) -> HashMap<String, String> {
    let mut aliases = HashMap::new();
    for athlete in athletes {
        let subject = athlete.id.as_str();
        let canonical = projection.canonical_id(subject);
        if canonical != subject {
            aliases.insert(subject.to_string(), canonical.to_string());
        }
    }
    aliases
}

fn coverage_evidence(snapshot: &census_store::StoreSnapshot<'_>) -> ReportResult<CoverageEvidence> {
    Ok(CoverageEvidence {
        rows: snapshot.scan(Table::Coverage)?,
        coach_lanes: snapshot.scan(Table::CoachLanes)?,
    })
}

fn school_map(rows: Vec<CanonicalSchool>) -> BTreeMap<SchoolId, CanonicalSchool> {
    rows.into_iter()
        .map(|school| (school.id.clone(), school))
        .collect()
}

fn team_map(rows: Vec<CanonicalTeam>) -> BTreeMap<TeamId, CanonicalTeam> {
    rows.into_iter()
        .map(|team| (team.id.clone(), team))
        .collect()
}

impl Drop for ExportDataset {
    fn drop(&mut self) {
        release_generation();
    }
}

impl ExportDataset {
    pub fn load(store: &Store) -> ReportResult<Self> {
        store.store_identity()?;
        let snapshot = store.snapshot();
        Self::from_snapshot(store, &snapshot)
    }

    pub fn from_snapshot(
        store: &Store,
        snapshot: &census_store::StoreSnapshot<'_>,
    ) -> ReportResult<Self> {
        snapshot.ensure_origin(store)?;
        snapshot.require_legacy_identity_publication()?;
        let loaded = read::all(snapshot)?;
        Self::from_loaded(loaded, frozen::lineage(store, snapshot)?)
    }

    pub fn for_job(store: &Store, job: &str) -> ReportResult<Self> {
        frozen::job::capture(store, job)
    }

    pub fn ensure_store(&self, store: &Store) -> ReportResult<()> {
        frozen::job::ensure_store(self, store)
    }

    pub fn ensure_current(&self, store: &Store) -> ReportResult<()> {
        frozen::ensure_current(self, store)
    }

    pub fn ensure_snapshot(&self, snapshot: &census_store::StoreSnapshot<'_>) -> ReportResult<()> {
        frozen::ensure_snapshot(self, snapshot)
    }

    fn from_loaded(loaded: Loaded, lineage: DatasetLineage) -> ReportResult<Self> {
        let identity_projection = projection_of(&loaded)?;
        let canonical_aliases = aliases_of(&loaded.athletes, &identity_projection);
        let mut dataset = Self {
            athletes: loaded.athletes,
            canonical_aliases,
            schools: school_map(loaded.schools),
            teams: team_map(loaded.teams),
            coaches: loaded.coaches,
            coach_observations: loaded.coach_observations,
            events: loaded.events,
            meets: loaded.meets,
            performances: loaded.performances,
            review_cases: loaded.review_cases,
            source_access: loaded.source_access,
            verdicts: loaded.verdicts,
            identity_decisions: loaded.identity_decisions,
            coverage_evidence: loaded.coverage_evidence,
            relays: loaded.relays,
            lineage,
            identity_projection,
        };
        note_generation();
        frozen::bind(&mut dataset)?;
        Ok(dataset)
    }

    pub fn save_frozen(&self, path: &std::path::Path) -> ReportResult<()> {
        frozen::save(self, path)
    }

    pub fn reopen_frozen(path: &std::path::Path) -> ReportResult<Self> {
        frozen::reopen(path)
    }

    pub(crate) fn archive_digest(&self) -> ReportResult<String> {
        frozen::archive_digest(self)
    }

    pub fn semantic_policy(&self) -> ReportResult<census_domain::model::EventSemantics> {
        frozen::semantic_policy(&self.lineage)
    }

    pub fn ensure_current_policy(&self) -> ReportResult<()> {
        frozen::ensure_current_policy(&self.lineage)
    }

    pub fn ensure_semantic_policy(&self) -> ReportResult<()> {
        frozen::ensure_semantic_policy(&self.lineage)
    }

    pub fn identities(&self) -> Arc<AthleteIdentityProjection> {
        Arc::clone(&self.identity_projection)
    }
}
