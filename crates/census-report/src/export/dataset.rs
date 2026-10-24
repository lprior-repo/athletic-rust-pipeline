use crate::report::ReportResult;
use census_domain::model::{
    AppliedAthleteIdentity, AthleteIdentityProjection, CanonicalAthlete, CanonicalCoach,
    CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool, CanonicalTeam, CensusRun,
    CoachLaneObligation, CoverageRow, GradYear, RelayResult, ReviewCase, ReviewVerdictRecord,
    SchoolId, SourceAccessCondition, TeamId,
};
use census_store::{Store, Table};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

mod frozen;
mod initial_objects;
mod job_request;
mod loaded;
pub use job_request::ExportJobRequest;
use loaded::{aliases_of, projection_of, Loaded};
mod read;
pub(crate) mod reported_profile_prs;
pub(crate) mod selected;
mod selected_proofs;

pub(crate) use frozen::lineage as store_snapshot_lineage;
pub(crate) use frozen::proof_objects;
pub use frozen::{CancelOutcome, FrozenRoot};

pub(crate) const MAX_FROZEN_INPUT_BYTES: u64 = 200 * 1024 * 1024 * 1024;

static RESIDENT_GENERATIONS: AtomicU64 = AtomicU64::new(0);
static PEAK_RESIDENT_GENERATIONS: AtomicU64 = AtomicU64::new(0);
static DECODED_GENERATIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(test)]
static GENERATION_COUNTER_SERIAL: std::sync::RwLock<()> = std::sync::RwLock::new(());

#[cfg(test)]
thread_local! {
    static IN_COUNTER_WINDOW: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) struct GenerationCounterWindow {
    _exclusive: std::sync::RwLockWriteGuard<'static, ()>,
}

#[cfg(test)]
impl Drop for GenerationCounterWindow {
    fn drop(&mut self) {
        IN_COUNTER_WINDOW.with(|flag| flag.set(false));
    }
}

#[cfg(test)]
pub(crate) fn generation_counter_window() -> Result<GenerationCounterWindow, String> {
    let exclusive = GENERATION_COUNTER_SERIAL
        .write()
        .map_err(|_| "generation counter serial window poisoned".to_string())?;
    IN_COUNTER_WINDOW.with(|flag| flag.set(true));
    Ok(GenerationCounterWindow {
        _exclusive: exclusive,
    })
}

#[cfg(test)]
fn in_counter_window() -> bool {
    IN_COUNTER_WINDOW.with(|flag| flag.get())
}

#[cfg(test)]
fn shared_counter_access() -> Option<std::sync::RwLockReadGuard<'static, ()>> {
    if in_counter_window() {
        return None;
    }
    Some(match GENERATION_COUNTER_SERIAL.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    })
}

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

use selected::SelectedInputOrigin;

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
    pub(crate) canonical_aliases: HashMap<String, String>,
    #[serde(skip)]
    pub lineage: DatasetLineage,
    pub coverage_evidence: CoverageEvidence,
    pub relays: Vec<RelayResult>,
    #[serde(skip_serializing_if = "reported_profile_prs::ReportedProfilePrs::is_empty")]
    reported_profile_prs: reported_profile_prs::ReportedProfilePrs,
    #[serde(skip)]
    identity_projection: Arc<AthleteIdentityProjection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selection: Option<super::qualification::SelectionBinding>,
    #[serde(skip)]
    verified_participation: Option<super::VerifiedParticipationView>,
    #[serde(skip)]
    verified_current_affiliations: Option<super::VerifiedCurrentAffiliations>,
    #[serde(skip)]
    verified_public_identities: Option<super::public_identity::VerifiedPublicIdentityView>,
    #[serde(skip)]
    selected_proofs: Option<selected_proofs::SelectedProofs>,
    #[serde(skip)]
    selected_input_origin: SelectedInputOrigin,
    #[serde(skip)]
    selected_job_freshness: Option<frozen::job::SelectedJobFreshness>,
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
    store_snapshot_lineage(store, &snapshot)
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
        #[cfg(test)]
        let _shared = shared_counter_access();
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

    fn from_loaded(loaded: Loaded, lineage: DatasetLineage) -> ReportResult<Self> {
        let identity_projection = projection_of(&loaded)?;
        let canonical_aliases = aliases_of(&loaded.athletes, &identity_projection);
        let mut dataset = Self {
            selection: loaded.selection,
            verified_participation: None,
            verified_current_affiliations: None,
            verified_public_identities: None,
            selected_proofs: None,
            selected_input_origin: SelectedInputOrigin::Unqualified,
            selected_job_freshness: None,
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
            reported_profile_prs: loaded.reported_profile_prs,
            lineage,
            identity_projection,
        };
        #[cfg(test)]
        let _shared = shared_counter_access();
        note_generation();
        frozen::bind(&mut dataset)?;
        Ok(dataset)
    }

    pub fn reopen_frozen(path: &std::path::Path) -> ReportResult<Self> {
        frozen::reopen(path)
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
