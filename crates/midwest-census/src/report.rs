//! Measured census report. Every number is computed from consolidated entity logs — never asserted
//! in prose — so the acceptance answers are reproducible from the store alone.
//!
//! The report deliberately deserializes the **canonical** entity types rather than mirroring their
//! wire shape: a hand-written mirror silently drifts from the model and would report on fields that
//! no longer exist.

use crate::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, Evidence, Gender, GradYear, SourceNamespace, Sport,
};
use crate::store::{Store, Table};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Stream a JSONL entity log, tolerating a truncated tail from an interrupted run.
///
/// This reads a *materialized snapshot*: the census itself reads the store through
/// [`Store::scan`], and this survives for callers that re-read an export they just wrote. Every
/// iteration consumes one line of a finite file, so the loop terminates on the line count.
pub fn read_rows<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Vec<T>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut rows = Vec::new();
    let mut unparseable = 0usize;
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(trimmed) {
            Ok(row) => rows.push(row),
            Err(error) => {
                unparseable = unparseable.saturating_add(1);
                anyhow::ensure!(
                    unparseable <= 1,
                    "unparseable row {} in {}: {error}",
                    index.saturating_add(1),
                    path.display()
                );
            }
        }
    }
    Ok(rows)
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct StateCensus {
    pub state: String,
    pub schools: usize,
    pub athletes: usize,
    pub class_of_2027: usize,
    pub class_of_2027_boys: usize,
    pub class_of_2027_girls: usize,
    pub class_of_2027_unknown_gender: usize,
    pub class_of_2027_with_profile_url: usize,
    pub class_of_2027_with_grad_year_evidence: usize,
    pub class_of_2027_multisource: usize,
    pub class_of_2027_with_coach: usize,
    pub class_of_2027_with_coach_email: usize,
    pub coaches: usize,
    pub coaches_with_email: usize,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct SportsBreakdown {
    pub indoor_only: usize,
    pub outdoor_only: usize,
    pub cross_country_only: usize,
    pub multi_sport: usize,
    pub none: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderCoverage {
    /// Source namespaces observed on class-of-2027 athletes (`milesplit_athlete`, …).
    pub namespaces: BTreeMap<String, usize>,
    /// Class-of-2027 athletes reachable through ≥2 distinct source namespaces.
    pub multisource_athletes: usize,
    /// Class-of-2027 athletes whose Athletic.net profile URL is already known without any Athletic.net
    /// request.
    pub athletic_net_urls_known: usize,
    /// `SourceRef::id` values that appear in observed-grade evidence for class-of-2027 athletes.
    pub grade_evidence_sources: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Census {
    pub generated_on: String,
    pub store_dir: String,
    /// `all_sources` or `core` — see [`Scope`].
    pub scope: String,
    pub totals: StateCensus,
    pub by_state: BTreeMap<String, StateCensus>,
    pub athletes_by_grad_year: BTreeMap<String, usize>,
    pub class_of_2027_sports: SportsBreakdown,
    pub providers: ProviderCoverage,
    pub coach_roles: BTreeMap<String, usize>,
    pub coach_sports: BTreeMap<String, usize>,
    pub coach_sources: BTreeMap<String, usize>,
    /// Meet coverage, including how many meets already name their Athletic.net counterpart.
    pub meets: MeetCoverage,
    pub duplicate_school_names: usize,
    pub notes: Vec<String>,
}

/// Meet-table coverage. `with_athletic_net_id` counts meets whose source identities include an
/// Athletic.net meet id, i.e. meets that can be requested from Athletic.net directly by id instead
/// of being discovered by enumeration.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MeetCoverage {
    pub total: usize,
    pub with_athletic_net_id: usize,
    pub by_state: BTreeMap<String, usize>,
    /// Timer/provider namespace slug (for example `timer_meet:live_results`) to meet count.
    pub by_provider: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_date: Option<String>,
}

fn is_track_or_xc(sport: Sport) -> bool {
    matches!(
        sport,
        Sport::OutdoorTrack | Sport::IndoorTrack | Sport::CrossCountry
    )
}

/// Report scope.
///
/// The platform's **core** deliberately excludes Athletic.net and the AthleticLIVE derivative: the
/// objective requires the core to work, and be measurable, with those adapters never registered.
/// Every core number therefore has to be reachable from association, MileSplit, official-artifact,
/// timer, or school-site evidence alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Every consolidated row, including AthleticLIVE enrichment.
    AllSources,
    /// Only entities with at least one evidence source that is not Athletic.net-derived.
    Core,
}

impl Scope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Scope::AllSources => "all_sources",
            Scope::Core => "core",
        }
    }

    /// `""` or `"-core"`, appended to `report.json` / `census-by-state.csv`.
    const fn file_suffix(self) -> &'static str {
        match self {
            Scope::AllSources => "",
            Scope::Core => "-core",
        }
    }
}

/// Adapter ids whose evidence does not count toward the core census.
///
/// `athleticlive_*` is the Athletic.net mirror (meet index, athlete rows) and `athleticnet` is the
/// host itself, read through the owner-authorized athlete-bio adapter. A core entity must be
/// reachable without any of them, so their evidence is ignored while the core filter runs.
pub const NON_CORE_SOURCE_IDS: [&str; 3] = [
    "athleticlive_athletes",
    "athleticlive_meets_csv",
    "athleticnet",
];

/// Entity tables a non-core adapter can populate.
pub trait CoreScoped {
    fn evidence(&self) -> &[Evidence];

    fn evidence_mut(&mut self) -> &mut Vec<Evidence>;

    /// Drop grade observations that came from a non-core source. No-op where a table has none.
    fn drop_non_core_observations(&mut self) {}

    /// Drop identities minted from a non-core namespace. No-op where a table has none.
    fn drop_non_core_identities(&mut self) {}
}

/// True when `id` is an adapter that belongs to the platform's own core.
pub fn is_core_source(id: &str) -> bool {
    !NON_CORE_SOURCE_IDS.contains(&id)
}

impl CoreScoped for CanonicalAthlete {
    fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    fn evidence_mut(&mut self) -> &mut Vec<Evidence> {
        &mut self.evidence
    }

    fn drop_non_core_observations(&mut self) {
        self.observed_grades
            .retain(|observation| is_core_source(&observation.source.id));
    }

    fn drop_non_core_identities(&mut self) {
        self.source_identities
            .retain(|identity| identity.namespace.is_core());
    }
}

impl CoreScoped for CanonicalMeet {
    fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    fn evidence_mut(&mut self) -> &mut Vec<Evidence> {
        &mut self.evidence
    }

    fn drop_non_core_identities(&mut self) {
        self.source_identities
            .retain(|identity| identity.namespace.is_core());
    }
}

/// An event carries evidence and per-source labels, and no identities: the default no-ops cover
/// everything but the evidence filter, which is the rule every other table follows.
impl CoreScoped for CanonicalEvent {
    fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    fn evidence_mut(&mut self) -> &mut Vec<Evidence> {
        &mut self.evidence
    }
}

/// A performance carries evidence, a bare grade, and a provider-local key; only the evidence can name
/// the adapter that produced the row.
impl CoreScoped for CanonicalPerformance {
    fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    fn evidence_mut(&mut self) -> &mut Vec<Evidence> {
        &mut self.evidence
    }
}

/// Reduce rows to what the core could know on its own.
///
/// Non-core evidence, grade observations, and identities are removed first, exactly as they would be
/// absent had the non-core adapters never been registered; a row left with no evidence at all is
/// then dropped. Returns the number of dropped rows.
pub fn retain_core<T: CoreScoped>(rows: &mut Vec<T>) -> usize {
    let before = rows.len();
    for row in rows.iter_mut() {
        row.evidence_mut()
            .retain(|evidence| is_core_source(&evidence.source.id));
        row.drop_non_core_observations();
        row.drop_non_core_identities();
    }
    rows.retain(|row| !row.evidence().is_empty());
    before.saturating_sub(rows.len())
}

/// Saturating counter bump.
///
/// Counters cannot exceed the scanned row count, which [`crate::store::MAX_ROWS_PER_TABLE`] bounds,
/// so saturation is unreachable in practice; it is here so a change to that bound can never wrap a
/// counter or trap the report.
fn bump(counter: &mut usize) {
    *counter = counter.saturating_add(1);
}

/// Saturating accumulation of `value` into `total`.
fn add(total: &mut usize, value: usize) {
    *total = total.saturating_add(value);
}

/// The per-state bucket for `state`, created on first use.
fn state_entry<'a>(
    by_state: &'a mut BTreeMap<String, StateCensus>,
    state: &str,
) -> &'a mut StateCensus {
    by_state
        .entry(state.to_string())
        .or_insert_with(|| StateCensus {
            state: state.to_string(),
            ..StateCensus::default()
        })
}

/// Row counts behind the `ALL` row and the report notes. `athletes` is measured after the [`Scope`]
/// filter, so the notes describe the rows the report actually used.
struct RowCounts {
    schools: usize,
    athletes: usize,
    coaches: usize,
    coaches_with_email: usize,
    dropped: usize,
}

impl RowCounts {
    fn of(
        schools: &[CanonicalSchool],
        athletes: &[CanonicalAthlete],
        coaches: &[CanonicalCoach],
        dropped: usize,
    ) -> Self {
        Self {
            schools: schools.len(),
            athletes: athletes.len(),
            coaches: coaches.len(),
            coaches_with_email: coaches
                .iter()
                .filter(|coach| coach.professional_email.is_some())
                .count(),
            dropped,
        }
    }
}

/// Class-of-2027 counters that are not per-state.
#[derive(Default)]
struct Co2027Rollup {
    sports: SportsBreakdown,
    namespaces: BTreeMap<String, usize>,
    grade_evidence_sources: BTreeMap<String, usize>,
    multisource: usize,
    athletic_net_urls: usize,
}

/// Athlete-derived census counters.
#[derive(Default)]
struct AthleteRollup {
    by_state: BTreeMap<String, StateCensus>,
    by_grad_year: BTreeMap<String, usize>,
    co2027: Co2027Rollup,
}

/// Coach-derived census counters.
#[derive(Default)]
struct CoachRollup {
    by_state: BTreeMap<String, (usize, usize)>,
    roles: BTreeMap<String, usize>,
    sports: BTreeMap<String, usize>,
    sources: BTreeMap<String, usize>,
}

/// The state of the school an entity belongs to, or `UNKNOWN` for an unknown school.
fn state_of(school_state: &HashMap<&str, &str>, school: &str) -> String {
    school_state
        .get(school)
        .copied()
        .unwrap_or("UNKNOWN")
        .to_string()
}

/// School id -> state of that school.
fn school_state_index(schools: &[CanonicalSchool]) -> HashMap<&str, &str> {
    schools
        .iter()
        .map(|school| {
            (
                school.id.as_str(),
                school.state.as_deref().unwrap_or("UNKNOWN"),
            )
        })
        .collect()
}

/// School id -> (a track/XC coach, whether any track/XC coach brings a professional email).
fn school_coach_index(coaches: &[CanonicalCoach]) -> HashMap<&str, (&CanonicalCoach, bool)> {
    let mut index: HashMap<&str, (&CanonicalCoach, bool)> = HashMap::new();
    for coach in coaches {
        if !coach.sport.is_some_and(is_track_or_xc) {
            continue;
        }
        let entry = index.entry(coach.school.as_str()).or_insert((coach, false));
        if coach.professional_email.is_some() {
            entry.1 = true;
        }
    }
    index
}

/// Meet-table coverage, one pass over the merged meet rows.
fn meet_coverage(meets: &[CanonicalMeet]) -> MeetCoverage {
    let mut coverage = MeetCoverage::default();
    for meet in meets {
        bump(&mut coverage.total);
        bump(coverage.by_state.entry(meet.state.clone()).or_default());
        let names_athletic_net = meet.source_identities.iter().any(|identity| {
            matches!(
                identity.namespace,
                SourceNamespace::LegacyAthleticNet { .. }
            )
        });
        if names_athletic_net {
            bump(&mut coverage.with_athletic_net_id);
        }
        for identity in &meet.source_identities {
            if let SourceNamespace::TimerMeet { provider } = &identity.namespace {
                let slug = format!("timer_meet:{provider}");
                bump(coverage.by_provider.entry(slug).or_default());
            }
        }
        let earliest = coverage.first_date.get_or_insert_with(|| meet.date.clone());
        if meet.date < *earliest {
            *earliest = meet.date.clone();
        }
        let latest = coverage.last_date.get_or_insert_with(|| meet.date.clone());
        if meet.date > *latest {
            *latest = meet.date.clone();
        }
    }
    coverage
}

/// Track/XC breakdown for one class-of-2027 athlete.
fn tally_sports(sports: &mut SportsBreakdown, athlete: &CanonicalAthlete) {
    let indoor = athlete.sports.contains(&Sport::IndoorTrack);
    let outdoor = athlete.sports.contains(&Sport::OutdoorTrack);
    let cross_country = athlete.sports.contains(&Sport::CrossCountry);
    match (indoor, outdoor, cross_country) {
        (false, false, false) => bump(&mut sports.none),
        (true, false, false) => bump(&mut sports.indoor_only),
        (false, true, false) => bump(&mut sports.outdoor_only),
        (false, false, true) => bump(&mut sports.cross_country_only),
        _ => bump(&mut sports.multi_sport),
    }
}

/// Class-of-2027 counting for one athlete, over exactly the cohort the per-state buckets count.
fn tally_co2027(
    entry: &mut StateCensus,
    co: &mut Co2027Rollup,
    athlete: &CanonicalAthlete,
    school_coach: &HashMap<&str, (&CanonicalCoach, bool)>,
) {
    bump(&mut entry.class_of_2027);
    match athlete.gender {
        Gender::Boys => bump(&mut entry.class_of_2027_boys),
        Gender::Girls => bump(&mut entry.class_of_2027_girls),
        Gender::Mixed | Gender::Unknown => bump(&mut entry.class_of_2027_unknown_gender),
    }
    if !athlete.public_profile_urls.is_empty() {
        bump(&mut entry.class_of_2027_with_profile_url);
    }
    if !athlete.observed_grades.is_empty() {
        bump(&mut entry.class_of_2027_with_grad_year_evidence);
    }
    for observation in &athlete.observed_grades {
        bump(
            co.grade_evidence_sources
                .entry(observation.source.id.clone())
                .or_default(),
        );
    }
    let distinct: BTreeSet<String> = athlete
        .source_identities
        .iter()
        .map(|identity| identity.namespace.to_string())
        .collect();
    if distinct.len() > 1 {
        bump(&mut co.multisource);
        bump(&mut entry.class_of_2027_multisource);
    }
    for namespace in distinct {
        bump(co.namespaces.entry(namespace).or_default());
    }
    if athlete
        .public_profile_urls
        .iter()
        .any(|url| url.contains("athletic.net"))
    {
        bump(&mut co.athletic_net_urls);
    }
    if let Some((_, has_email)) = school_coach.get(athlete.school.as_str()) {
        bump(&mut entry.class_of_2027_with_coach);
        if *has_email {
            bump(&mut entry.class_of_2027_with_coach_email);
        }
    }
    tally_sports(&mut co.sports, athlete);
}

/// One pass over the merged athlete rows.
fn rollup_athletes(
    athletes: &[CanonicalAthlete],
    school_state: &HashMap<&str, &str>,
    school_coach: &HashMap<&str, (&CanonicalCoach, bool)>,
) -> AthleteRollup {
    let mut rollup = AthleteRollup::default();
    for athlete in athletes {
        let state = state_of(school_state, athlete.school.as_str());
        bump(
            rollup
                .by_grad_year
                .entry(athlete.grad_year.to_string())
                .or_default(),
        );
        let entry = state_entry(&mut rollup.by_state, &state);
        bump(&mut entry.athletes);
        if athlete.grad_year == GradYear::CO2027 {
            tally_co2027(entry, &mut rollup.co2027, athlete, school_coach);
        }
    }
    rollup
}

/// `school_wide` for a coach who covers a whole school, else the lowercased sport name.
fn coach_sport(coach: &CanonicalCoach) -> String {
    coach
        .sport
        .map(|sport| format!("{sport:?}").to_lowercase())
        .unwrap_or_else(|| "school_wide".to_string())
}

/// One pass over the merged coach rows.
fn rollup_coaches(coaches: &[CanonicalCoach], school_state: &HashMap<&str, &str>) -> CoachRollup {
    let mut rollup = CoachRollup::default();
    for coach in coaches {
        let state = state_of(school_state, coach.school.as_str());
        let slot = rollup.by_state.entry(state).or_insert((0, 0));
        slot.0 = slot.0.saturating_add(1);
        if coach.professional_email.is_some() {
            slot.1 = slot.1.saturating_add(1);
        }
        let role = format!("{:?}", coach.role).to_lowercase();
        bump(rollup.roles.entry(role).or_default());
        bump(rollup.sports.entry(coach_sport(coach)).or_default());
        for identity in &coach.source_identities {
            bump(
                rollup
                    .sources
                    .entry(identity.namespace.to_string())
                    .or_default(),
            );
        }
    }
    rollup
}

/// Merge per-state coach counts into the athlete-derived buckets, creating a state that only a coach
/// mentions.
fn apply_coach_states(
    by_state: &mut BTreeMap<String, StateCensus>,
    coach_states: &BTreeMap<String, (usize, usize)>,
) {
    for (state, (total, with_email)) in coach_states {
        let entry = state_entry(by_state, state);
        entry.coaches = *total;
        entry.coaches_with_email = *with_email;
    }
}

/// School counts per state, from the school table rather than from athlete-derived buckets.
fn schools_by_state(schools: &[CanonicalSchool]) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for school in schools {
        let state = school
            .state
            .clone()
            .unwrap_or_else(|| "UNKNOWN".to_string());
        bump(counts.entry(state).or_default());
    }
    counts
}

/// Number of `(state, normalized_name)` pairs shared by more than one school row.
fn duplicate_school_names(schools: &[CanonicalSchool]) -> usize {
    let mut name_counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for school in schools {
        let key = (
            school.state.clone().unwrap_or_default(),
            school.normalized_name.clone(),
        );
        bump(name_counts.entry(key).or_default());
    }
    name_counts.values().filter(|count| **count > 1).count()
}

/// The `ALL` row: state buckets summed, school and coach totals taken from the tables.
fn totals_of(by_state: &BTreeMap<String, StateCensus>, counts: &RowCounts) -> StateCensus {
    let mut totals = StateCensus::default();
    for entry in by_state.values() {
        add(&mut totals.athletes, entry.athletes);
        add(&mut totals.class_of_2027, entry.class_of_2027);
        add(&mut totals.class_of_2027_boys, entry.class_of_2027_boys);
        add(&mut totals.class_of_2027_girls, entry.class_of_2027_girls);
        add(
            &mut totals.class_of_2027_unknown_gender,
            entry.class_of_2027_unknown_gender,
        );
        add(
            &mut totals.class_of_2027_with_profile_url,
            entry.class_of_2027_with_profile_url,
        );
        add(
            &mut totals.class_of_2027_with_grad_year_evidence,
            entry.class_of_2027_with_grad_year_evidence,
        );
        add(
            &mut totals.class_of_2027_multisource,
            entry.class_of_2027_multisource,
        );
        add(
            &mut totals.class_of_2027_with_coach,
            entry.class_of_2027_with_coach,
        );
        add(
            &mut totals.class_of_2027_with_coach_email,
            entry.class_of_2027_with_coach_email,
        );
    }
    totals.state = "TOTAL".to_string();
    totals.schools = counts.schools;
    totals.coaches = counts.coaches;
    totals.coaches_with_email = counts.coaches_with_email;
    totals
}

/// Provenance notes recorded in `report.json`.
fn census_notes(
    scope: Scope,
    counts: &RowCounts,
    coach_sources_empty: bool,
    out: &Path,
) -> Vec<String> {
    let mut notes = vec![format!(
        "scope={} schools={} athletes={} coaches={} from {}",
        scope.as_str(),
        counts.schools,
        counts.athletes,
        counts.coaches,
        out.display()
    )];
    if scope == Scope::Core {
        notes.push(format!(
            "core scope drops {} rows whose only evidence is {}",
            counts.dropped,
            NON_CORE_SOURCE_IDS.join("/")
        ));
    }
    if counts.athletes == 0 {
        notes.push(
            "no athletes consolidated yet — run `collect` then `consolidate` before `report`"
                .to_string(),
        );
    }
    if counts.coaches > 0 && coach_sources_empty {
        notes.push("coaches present but no coach source identities recorded".to_string());
    }
    notes
}

/// Build the census from the store's merged entity tables.
///
/// Every row comes from [`Store::scan`], which merges the append-only observations of a table into
/// one entity per id, so the report never depends on a materialized `out/*.jsonl` export. A scan
/// bounds a table at [`crate::store::MAX_ROWS_PER_TABLE`] observations, which is the bound every
/// loop below runs under.
pub fn build_census(store: &Store, scope: Scope) -> Result<Census> {
    let out = store.out_dir();
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
    let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    let dropped = match scope {
        Scope::AllSources => 0,
        Scope::Core => retain_core(&mut athletes).saturating_add(retain_core(&mut meets)),
    };
    let counts = RowCounts::of(&schools, &athletes, &coaches, dropped);
    let school_state = school_state_index(&schools);
    let school_coach = school_coach_index(&coaches);
    let athlete_rollup = rollup_athletes(&athletes, &school_state, &school_coach);
    let coach_rollup = rollup_coaches(&coaches, &school_state);
    let coach_sources_empty = coach_rollup.sources.is_empty();
    let mut by_state = athlete_rollup.by_state;
    apply_coach_states(&mut by_state, &coach_rollup.by_state);

    // Both the workbook and the CSV read a state's school count off its `by_state` row, so it is
    // filled once here from the school table rather than re-derived by each writer.
    let school_counts = schools_by_state(&schools);
    for (state, entry) in by_state.iter_mut() {
        entry.schools = school_counts.get(state).copied().unwrap_or(0);
    }

    Ok(Census {
        generated_on: crate::net::today_iso(),
        store_dir: store.root().display().to_string(),
        scope: scope.as_str().to_string(),
        totals: totals_of(&by_state, &counts),
        by_state,
        athletes_by_grad_year: athlete_rollup.by_grad_year,
        class_of_2027_sports: athlete_rollup.co2027.sports,
        providers: ProviderCoverage {
            namespaces: athlete_rollup.co2027.namespaces,
            multisource_athletes: athlete_rollup.co2027.multisource,
            athletic_net_urls_known: athlete_rollup.co2027.athletic_net_urls,
            grade_evidence_sources: athlete_rollup.co2027.grade_evidence_sources,
        },
        coach_roles: coach_rollup.roles,
        coach_sports: coach_rollup.sports,
        coach_sources: coach_rollup.sources,
        meets: meet_coverage(&meets),
        duplicate_school_names: duplicate_school_names(&schools),
        notes: census_notes(scope, &counts, coach_sources_empty, &out),
    })
}

/// Write the census JSON and a flat per-state CSV next to the consolidated logs.
pub fn write_census(
    store: &Store,
    census: &Census,
    scope: Scope,
) -> Result<(std::path::PathBuf, std::path::PathBuf)> {
    let out = store.out_dir();
    std::fs::create_dir_all(&out)?;
    let json_path = out.join(format!("report{}.json", scope.file_suffix()));
    std::fs::write(&json_path, serde_json::to_vec_pretty(census)?)?;
    let csv_path = out.join(format!("census-by-state{}.csv", scope.file_suffix()));
    let mut csv = String::from(
        "state,schools,athletes,co2027,co2027_boys,co2027_girls,co2027_profile_url,co2027_grade_evidence,co2027_multisource,co2027_with_coach,co2027_with_coach_email,coaches,coaches_with_email\n",
    );
    let mut rows: Vec<&StateCensus> = census.by_state.values().collect();
    rows.sort_by_key(|row| std::cmp::Reverse(row.class_of_2027));
    for row in rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            row.state,
            row.schools,
            row.athletes,
            row.class_of_2027,
            row.class_of_2027_boys,
            row.class_of_2027_girls,
            row.class_of_2027_with_profile_url,
            row.class_of_2027_with_grad_year_evidence,
            row.class_of_2027_multisource,
            row.class_of_2027_with_coach,
            row.class_of_2027_with_coach_email,
            row.coaches,
            row.coaches_with_email
        ));
    }
    csv.push_str(&format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        census.totals.state,
        census.totals.schools,
        census.totals.athletes,
        census.totals.class_of_2027,
        census.totals.class_of_2027_boys,
        census.totals.class_of_2027_girls,
        census.totals.class_of_2027_with_profile_url,
        census.totals.class_of_2027_with_grad_year_evidence,
        census.totals.class_of_2027_multisource,
        census.totals.class_of_2027_with_coach,
        census.totals.class_of_2027_with_coach_email,
        census.totals.coaches,
        census.totals.coaches_with_email
    ));
    std::fs::write(&csv_path, csv)?;
    Ok((json_path, csv_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CanonicalAthlete, CanonicalCoach, CanonicalSchool, CoachRole, Gender, GradYear,
        SourceIdentity, SourceNamespace, Sport,
    };
    #[test]
    fn core_scope_keeps_only_non_athletic_net_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let (school, school_id) = CanonicalSchool::new("WI", "Abbotsford", "abbotsford");
        store.append(Table::Schools, &school).unwrap();

        // One athlete reachable only through the AthleticLIVE mirror, one only through the
        // Athletic.net host adapter, and one through MileSplit.
        let mut mirrored =
            CanonicalAthlete::new(&school_id, "Mirror Only", GradYear::CO2027, Gender::Boys);
        mirrored.evidence.push(Evidence::parsed(
            crate::model::SourceRef::new("athleticlive_athletes", None),
            "2026-09-20",
        ));
        let mut host =
            CanonicalAthlete::new(&school_id, "Host Only", GradYear::CO2027, Gender::Boys);
        host.evidence.push(Evidence::parsed(
            crate::model::SourceRef::new("athleticnet", None),
            "2026-09-20",
        ));
        let mut core_athlete =
            CanonicalAthlete::new(&school_id, "Core Athlete", GradYear::CO2027, Gender::Boys);
        core_athlete.evidence.push(Evidence::parsed(
            crate::model::SourceRef::new("athleticlive_athletes", None),
            "2026-09-20",
        ));
        core_athlete.evidence.push(Evidence::parsed(
            crate::model::SourceRef::new("milesplit_roster", None),
            "2026-09-20",
        ));
        store.append(Table::Athletes, &mirrored).unwrap();
        store.append(Table::Athletes, &host).unwrap();
        store.append(Table::Athletes, &core_athlete).unwrap();

        let mut rows: Vec<CanonicalAthlete> = vec![mirrored, host, core_athlete];
        assert_eq!(retain_core(&mut rows), 2);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].canonical_name, "Core Athlete");
    }

    #[test]
    fn census_counts_class_of_2027_with_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let (school, school_id) = CanonicalSchool::new("WI", "Abbotsford", "abbotsford");
        store.append(Table::Schools, &school).unwrap();
        let mut athlete = CanonicalAthlete::new(
            &school_id,
            "Julian Aguilera",
            GradYear::CO2027,
            Gender::Boys,
        );
        athlete.source_identities.push(SourceIdentity::new(
            SourceNamespace::MilesplitAthlete,
            "14399169",
        ));
        athlete
            .public_profile_urls
            .push("https://wi.milesplit.com/athletes/14399169-julian-aguilera".to_string());
        athlete.sports.push(Sport::OutdoorTrack);
        store.append(Table::Athletes, &athlete).unwrap();
        let mut coach = CanonicalCoach::new(
            &school_id,
            "Dana Coach",
            Some(Sport::OutdoorTrack),
            Gender::Mixed,
            CoachRole::HeadCoach,
        );
        coach.professional_email = Some("coach@example.org".to_string());
        store.append(Table::Coaches, &coach).unwrap();

        // No consolidation step: the census reads the entity tables through the store.
        let census = build_census(&store, Scope::AllSources).unwrap();
        assert_eq!(census.totals.class_of_2027, 1);
        assert_eq!(census.totals.class_of_2027_boys, 1);
        assert_eq!(census.totals.class_of_2027_with_profile_url, 1);
        assert_eq!(census.totals.class_of_2027_with_coach, 1);
        assert_eq!(census.totals.class_of_2027_with_coach_email, 1);
        assert_eq!(census.by_state.get("WI").unwrap().class_of_2027, 1);
        assert_eq!(census.class_of_2027_sports.outdoor_only, 1);
        assert_eq!(
            census.providers.namespaces.get("milesplit_athlete"),
            Some(&1)
        );
        let (json_path, csv_path) = write_census(&store, &census, Scope::AllSources).unwrap();
        assert!(json_path.exists() && csv_path.exists());
    }

    #[test]
    fn census_reads_merged_observations_without_consolidating() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let (school, school_id) = CanonicalSchool::new("WI", "Abbotsford", "abbotsford");
        store.append(Table::Schools, &school).unwrap();
        let athlete = CanonicalAthlete::new(
            &school_id,
            "Julian Aguilera",
            GradYear::CO2027,
            Gender::Boys,
        );
        // The same entity appended twice and never consolidated: the report still sees one athlete.
        store.append(Table::Athletes, &athlete).unwrap();
        store.append(Table::Athletes, &athlete).unwrap();

        let census = build_census(&store, Scope::AllSources).unwrap();
        assert_eq!(census.totals.athletes, 1);
        assert_eq!(census.totals.class_of_2027, 1);
        assert_eq!(census.totals.schools, 1);
        // The workbook prints a state's school count off its `by_state` row, so the row has to carry
        // the count and not just the totals.
        assert_eq!(census.by_state["WI"].schools, 1);
        assert!(!store.out_dir().join("athletes.jsonl").exists());
    }
}
