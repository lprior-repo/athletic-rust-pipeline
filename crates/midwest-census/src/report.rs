//! Measured census report. Every number is computed from consolidated entity logs — never asserted
//! in prose — so the acceptance answers are reproducible from the store alone.
//!
//! The report deliberately deserializes the **canonical** entity types rather than mirroring their
//! wire shape: a hand-written mirror silently drifts from the model and would report on fields that
//! no longer exist.

use crate::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool, Evidence, Gender, GradYear,
    SourceNamespace, Sport,
};
use crate::store::Store;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Stream a JSONL entity log, tolerating a truncated tail from an interrupted run.
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
                unparseable += 1;
                anyhow::ensure!(
                    unparseable <= 1,
                    "unparseable row {} in {}: {error}",
                    index + 1,
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
    /// School counts per state, taken from the school table (a school can exist with no athletes).
    pub schools_by_state: BTreeMap<String, usize>,
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
/// `athleticlive_*` is the Athletic.net mirror (meet index, athlete rows). A core entity must be
/// reachable without them, so their evidence is ignored while the core filter runs.
pub const NON_CORE_SOURCE_IDS: [&str; 2] = ["athleticlive_athletes", "athleticlive_meets_csv"];

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
    before - rows.len()
}

/// Build the census from consolidated logs under `<store>/out/`.
pub fn build_census(store: &Store, scope: Scope) -> Result<Census> {
    let out = store.out_dir();
    let schools: Vec<CanonicalSchool> = read_rows(&out.join("schools.jsonl"))?;
    let mut athletes: Vec<CanonicalAthlete> = read_rows(&out.join("athletes.jsonl"))?;
    let coaches: Vec<CanonicalCoach> = read_rows(&out.join("coaches.jsonl"))?;
    let mut meets: Vec<CanonicalMeet> = read_rows(&out.join("meets.jsonl"))?;
    let dropped = match scope {
        Scope::AllSources => 0,
        Scope::Core => retain_core(&mut athletes) + retain_core(&mut meets),
    };

    let mut meet_coverage = MeetCoverage::default();
    for meet in &meets {
        meet_coverage.total += 1;
        *meet_coverage
            .by_state
            .entry(meet.state.clone())
            .or_insert(0) += 1;
        if meet.source_identities.iter().any(|identity| {
            matches!(
                identity.namespace,
                SourceNamespace::LegacyAthleticNet { .. }
            )
        }) {
            meet_coverage.with_athletic_net_id += 1;
        }
        for identity in &meet.source_identities {
            if let SourceNamespace::TimerMeet { provider } = &identity.namespace {
                *meet_coverage
                    .by_provider
                    .entry(format!("timer_meet:{provider}"))
                    .or_insert(0) += 1;
            }
        }
        let earliest = meet_coverage
            .first_date
            .get_or_insert_with(|| meet.date.clone());
        if meet.date < *earliest {
            *earliest = meet.date.clone();
        }
        let latest = meet_coverage
            .last_date
            .get_or_insert_with(|| meet.date.clone());
        if meet.date > *latest {
            *latest = meet.date.clone();
        }
    }

    let school_state: HashMap<&str, &str> = schools
        .iter()
        .map(|school| {
            (
                school.id.as_str(),
                school.state.as_deref().unwrap_or("UNKNOWN"),
            )
        })
        .collect();
    // A school "has a coach" when any coach record covers it for a track/XC sport.
    let mut school_coach: HashMap<&str, (&CanonicalCoach, bool)> = HashMap::new();
    for coach in &coaches {
        if !coach.sport.is_some_and(is_track_or_xc) {
            continue;
        }
        let entry = school_coach
            .entry(coach.school.as_str())
            .or_insert((coach, false));
        if coach.professional_email.is_some() {
            entry.1 = true;
        }
    }

    let mut by_state: BTreeMap<String, StateCensus> = BTreeMap::new();
    let mut athletes_by_grad_year: BTreeMap<String, usize> = BTreeMap::new();
    let mut sports = SportsBreakdown::default();
    let mut namespaces: BTreeMap<String, usize> = BTreeMap::new();
    let mut grade_evidence_sources: BTreeMap<String, usize> = BTreeMap::new();
    let mut multisource = 0usize;
    let mut athletic_net_urls = 0usize;

    for athlete in &athletes {
        let state = school_state
            .get(athlete.school.as_str())
            .copied()
            .unwrap_or("UNKNOWN")
            .to_string();
        *athletes_by_grad_year
            .entry(athlete.grad_year.to_string())
            .or_default() += 1;
        let entry = by_state
            .entry(state.clone())
            .or_insert_with(|| StateCensus {
                state: state.clone(),
                ..StateCensus::default()
            });
        entry.athletes += 1;

        if athlete.grad_year != GradYear::CO2027 {
            continue;
        }
        entry.class_of_2027 += 1;
        match athlete.gender {
            Gender::Boys => entry.class_of_2027_boys += 1,
            Gender::Girls => entry.class_of_2027_girls += 1,
            Gender::Mixed | Gender::Unknown => entry.class_of_2027_unknown_gender += 1,
        }
        if !athlete.public_profile_urls.is_empty() {
            entry.class_of_2027_with_profile_url += 1;
        }
        if !athlete.observed_grades.is_empty() {
            entry.class_of_2027_with_grad_year_evidence += 1;
        }
        for observation in &athlete.observed_grades {
            *grade_evidence_sources
                .entry(observation.source.id.clone())
                .or_default() += 1;
        }
        let distinct: BTreeSet<String> = athlete
            .source_identities
            .iter()
            .map(|identity| identity.namespace.to_string())
            .collect();
        if distinct.len() > 1 {
            multisource += 1;
            entry.class_of_2027_multisource += 1;
        }
        for namespace in distinct {
            *namespaces.entry(namespace).or_default() += 1;
        }
        if athlete
            .public_profile_urls
            .iter()
            .any(|url| url.contains("athletic.net"))
        {
            athletic_net_urls += 1;
        }
        if let Some((_, has_email)) = school_coach.get(athlete.school.as_str()) {
            entry.class_of_2027_with_coach += 1;
            if *has_email {
                entry.class_of_2027_with_coach_email += 1;
            }
        }
        let indoor = athlete.sports.contains(&Sport::IndoorTrack);
        let outdoor = athlete.sports.contains(&Sport::OutdoorTrack);
        let xc = athlete.sports.contains(&Sport::CrossCountry);
        match (indoor, outdoor, xc) {
            (false, false, false) => sports.none += 1,
            (true, false, false) => sports.indoor_only += 1,
            (false, true, false) => sports.outdoor_only += 1,
            (false, false, true) => sports.cross_country_only += 1,
            _ => sports.multi_sport += 1,
        }
    }

    let mut coaches_by_state: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut coach_roles: BTreeMap<String, usize> = BTreeMap::new();
    let mut coach_sports: BTreeMap<String, usize> = BTreeMap::new();
    let mut coach_sources: BTreeMap<String, usize> = BTreeMap::new();
    for coach in &coaches {
        let state = school_state
            .get(coach.school.as_str())
            .copied()
            .unwrap_or("UNKNOWN")
            .to_string();
        let slot = coaches_by_state.entry(state).or_insert((0, 0));
        slot.0 += 1;
        if coach.professional_email.is_some() {
            slot.1 += 1;
        }
        *coach_roles
            .entry(format!("{:?}", coach.role).to_lowercase())
            .or_default() += 1;
        *coach_sports
            .entry(
                coach
                    .sport
                    .map(|sport| format!("{sport:?}").to_lowercase())
                    .unwrap_or_else(|| "school_wide".to_string()),
            )
            .or_default() += 1;
        for identity in &coach.source_identities {
            *coach_sources
                .entry(identity.namespace.to_string())
                .or_default() += 1;
        }
    }
    for (state, (total, with_email)) in coaches_by_state {
        let entry = by_state
            .entry(state.clone())
            .or_insert_with(|| StateCensus {
                state: state.clone(),
                ..StateCensus::default()
            });
        entry.coaches = total;
        entry.coaches_with_email = with_email;
    }

    let mut schools_by_state: BTreeMap<String, usize> = BTreeMap::new();
    for school in &schools {
        *schools_by_state
            .entry(
                school
                    .state
                    .clone()
                    .unwrap_or_else(|| "UNKNOWN".to_string()),
            )
            .or_default() += 1;
    }

    let mut name_counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for school in &schools {
        *name_counts
            .entry((
                school.state.clone().unwrap_or_default(),
                school.normalized_name.clone(),
            ))
            .or_default() += 1;
    }
    let duplicate_school_names = name_counts.values().filter(|count| **count > 1).count();

    let mut totals = StateCensus::default();
    for entry in by_state.values() {
        totals.athletes += entry.athletes;
        totals.class_of_2027 += entry.class_of_2027;
        totals.class_of_2027_boys += entry.class_of_2027_boys;
        totals.class_of_2027_girls += entry.class_of_2027_girls;
        totals.class_of_2027_unknown_gender += entry.class_of_2027_unknown_gender;
        totals.class_of_2027_with_profile_url += entry.class_of_2027_with_profile_url;
        totals.class_of_2027_with_grad_year_evidence += entry.class_of_2027_with_grad_year_evidence;
        totals.class_of_2027_multisource += entry.class_of_2027_multisource;
        totals.class_of_2027_with_coach += entry.class_of_2027_with_coach;
        totals.class_of_2027_with_coach_email += entry.class_of_2027_with_coach_email;
    }
    totals.state = "ALL".to_string();
    totals.schools = schools.len();
    totals.coaches = coaches.len();
    totals.coaches_with_email = coaches
        .iter()
        .filter(|coach| coach.professional_email.is_some())
        .count();

    let mut notes = Vec::new();
    notes.push(format!(
        "scope={} schools={} athletes={} coaches={} from {}",
        scope.as_str(),
        schools.len(),
        athletes.len(),
        coaches.len(),
        out.display()
    ));
    if scope == Scope::Core {
        notes.push(format!(
            "core scope drops {dropped} rows whose only evidence is {}",
            NON_CORE_SOURCE_IDS.join("/")
        ));
    }
    if athletes.is_empty() {
        notes.push(
            "no athletes consolidated yet — run `collect` then `consolidate` before `report`"
                .to_string(),
        );
    }
    if !coaches.is_empty() && coach_sources.is_empty() {
        notes.push("coaches present but no coach source identities recorded".to_string());
    }

    Ok(Census {
        generated_on: crate::net::today_iso(),
        store_dir: store.root().display().to_string(),
        scope: scope.as_str().to_string(),
        totals,
        by_state,
        schools_by_state,
        athletes_by_grad_year,
        class_of_2027_sports: sports,
        providers: ProviderCoverage {
            namespaces,
            multisource_athletes: multisource,
            athletic_net_urls_known: athletic_net_urls,
            grade_evidence_sources,
        },
        coach_roles,
        coach_sports,
        coach_sources,
        meets: meet_coverage,
        duplicate_school_names,
        notes,
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
    rows.sort_by(|a, b| b.class_of_2027.cmp(&a.class_of_2027));
    for row in rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            row.state,
            schools_of(census, &row.state),
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
        "ALL,{},{},{},{},{},{},{},{},{},{},{},{}\n",
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

/// Per-state school counts come from the school table, not from athlete-derived state buckets.
fn schools_of(census: &Census, state: &str) -> usize {
    census
        .schools_by_state
        .get(state)
        .copied()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CanonicalAthlete, CanonicalCoach, CanonicalSchool, CoachRole, Gender, GradYear,
        SourceIdentity, SourceNamespace, Sport,
    };
    use crate::store::Table;

    #[test]
    fn core_scope_keeps_only_non_athletic_net_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let (school, school_id) = CanonicalSchool::new("WI", "Abbotsford", "abbotsford");
        store.append(Table::Schools, &school).unwrap();

        // One athlete reachable only through the AthleticLIVE mirror, one through MileSplit.
        let mut mirrored =
            CanonicalAthlete::new(&school_id, "Mirror Only", GradYear::CO2027, Gender::Boys);
        mirrored.evidence.push(Evidence::parsed(
            crate::model::SourceRef::new("athleticlive_athletes", None),
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
        store.append(Table::Athletes, &core_athlete).unwrap();

        let mut rows: Vec<CanonicalAthlete> = vec![mirrored, core_athlete];
        assert_eq!(retain_core(&mut rows), 1);
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

        store
            .consolidate::<CanonicalSchool>(Table::Schools, &store.out_dir().join("schools.jsonl"))
            .unwrap();
        store
            .consolidate::<CanonicalAthlete>(
                Table::Athletes,
                &store.out_dir().join("athletes.jsonl"),
            )
            .unwrap();
        store
            .consolidate::<CanonicalCoach>(Table::Coaches, &store.out_dir().join("coaches.jsonl"))
            .unwrap();

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
}
