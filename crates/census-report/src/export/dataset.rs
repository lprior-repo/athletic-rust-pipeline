use crate::report::{ReportError, ReportResult};
use census_domain::model::{
    AppliedAthleteIdentity, AthleteIdentityIndex, AthleteIdentityProjection, CanonicalAthlete,
    CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, ReviewCase, ReviewVerdictRecord, SchoolId, SourceAccessCondition, TeamId,
};
use census_store::clock::{Clock, SystemClock};
use census_store::{Store, Table};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

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
    pub canonical_aliases: HashMap<String, String>,
    pub lineage: DatasetLineage,
    identity_projection: Arc<AthleteIdentityProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetLineage {
    pub store_root: String,
    pub generated_on: String,
}

struct Loaded {
    athletes: Vec<CanonicalAthlete>,
    schools: Vec<CanonicalSchool>,
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
}

fn read_table<T: census_store::Entity>(
    snapshot: &census_store::StoreSnapshot<'_>,
    table: Table,
) -> ReportResult<Vec<T>> {
    Ok(snapshot.scan(table)?)
}

fn read_coach_observations(
    snapshot: &census_store::StoreSnapshot<'_>,
) -> ReportResult<Vec<CanonicalCoach>> {
    let mut rows = Vec::new();
    snapshot.for_each_observation(Table::Coaches, |mut coach: CanonicalCoach| {
        census_store::Entity::publish(&mut coach);
        rows.push(coach);
        Ok(())
    })?;
    Ok(rows)
}

fn join<T>(
    handle: std::thread::ScopedJoinHandle<'_, ReportResult<T>>,
    name: &str,
) -> ReportResult<T> {
    match handle.join() {
        Ok(result) => result,
        Err(_) => Err(ReportError::Invariant {
            detail: format!("reading {name} from the store panicked"),
        }),
    }
}

fn read_all(snapshot: &census_store::StoreSnapshot<'_>) -> ReportResult<Loaded> {
    std::thread::scope(|scope| -> ReportResult<Loaded> {
        let athletes = scope.spawn(|| Ok(snapshot.athletes()?));
        let schools = scope.spawn(|| read_table::<CanonicalSchool>(snapshot, Table::Schools));
        let teams = scope.spawn(|| read_table::<CanonicalTeam>(snapshot, Table::Teams));
        let coaches = scope.spawn(|| read_table::<CanonicalCoach>(snapshot, Table::Coaches));
        let coach_observations = scope.spawn(|| read_coach_observations(snapshot));
        let events = scope.spawn(|| read_table::<CanonicalEvent>(snapshot, Table::Events));
        let meets = scope.spawn(|| read_table::<CanonicalMeet>(snapshot, Table::Meets));
        let performances =
            scope.spawn(|| read_table::<CanonicalPerformance>(snapshot, Table::Performances));
        let review_cases = scope.spawn(|| read_table::<ReviewCase>(snapshot, Table::ReviewCases));
        let source_access =
            scope.spawn(|| read_table::<SourceAccessCondition>(snapshot, Table::SourceAccess));
        let verdicts =
            scope.spawn(|| read_table::<ReviewVerdictRecord>(snapshot, Table::IdentityVerdicts));
        let identity_decisions = scope.spawn(|| {
            read_table::<AppliedAthleteIdentity>(snapshot, Table::AthleteIdentityDecisions)
        });
        let joined = (
            join(athletes, "athletes"),
            join(schools, "schools"),
            join(teams, "teams"),
            join(coaches, "coaches"),
            join(coach_observations, "coach observations"),
            join(events, "events"),
            join(meets, "meets"),
            join(performances, "performances"),
            join(review_cases, "review cases"),
            join(source_access, "source access"),
            join(verdicts, "verdicts"),
            join(identity_decisions, "identity decisions"),
        );
        Ok(Loaded {
            athletes: joined.0?,
            schools: joined.1?,
            teams: joined.2?,
            coaches: joined.3?,
            coach_observations: joined.4?,
            events: joined.5?,
            meets: joined.6?,
            performances: joined.7?,
            review_cases: joined.8?,
            source_access: joined.9?,
            verdicts: joined.10?,
            identity_decisions: joined.11?,
        })
    })
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

impl ExportDataset {
    pub fn load(store: &Store) -> ReportResult<Self> {
        let snapshot = store.snapshot();
        let loaded = read_all(&snapshot)?;
        let identity_projection = projection_of(&loaded)?;
        let canonical_aliases = aliases_of(&loaded.athletes, &identity_projection);
        Ok(Self {
            athletes: loaded.athletes,
            canonical_aliases,
            schools: loaded
                .schools
                .into_iter()
                .map(|school| (school.id.clone(), school))
                .collect(),
            teams: loaded
                .teams
                .into_iter()
                .map(|team| (team.id.clone(), team))
                .collect(),
            coaches: loaded.coaches,
            coach_observations: loaded.coach_observations,
            events: loaded.events,
            meets: loaded.meets,
            performances: loaded.performances,
            review_cases: loaded.review_cases,
            source_access: loaded.source_access,
            verdicts: loaded.verdicts,
            identity_decisions: loaded.identity_decisions,
            lineage: DatasetLineage {
                store_root: store.root().display().to_string(),
                generated_on: SystemClock.today(),
            },
            identity_projection,
        })
    }

    pub fn identities(&self) -> Arc<AthleteIdentityProjection> {
        Arc::clone(&self.identity_projection)
    }
}
