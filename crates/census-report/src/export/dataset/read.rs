use super::Loaded;
use crate::report::{ReportError, ReportResult};
use census_domain::model::{
    AppliedAthleteIdentity, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, ReviewCase, ReviewVerdictRecord,
    SourceAccessCondition,
};
use census_store::{StoreSnapshot, Table};

pub(super) fn all(snapshot: &StoreSnapshot<'_>) -> ReportResult<Loaded> {
    std::thread::scope(|scope| ReadTasks::spawn(scope, snapshot).join())
}

fn table<T: census_store::Entity>(
    snapshot: &StoreSnapshot<'_>,
    table: Table,
) -> ReportResult<Vec<T>> {
    Ok(snapshot.scan(table)?)
}

fn coach_observations(snapshot: &StoreSnapshot<'_>) -> ReportResult<Vec<CanonicalCoach>> {
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

type ReadTask<'a, T> = std::thread::ScopedJoinHandle<'a, ReportResult<Vec<T>>>;
type ReadResults = (
    ReportResult<Vec<CanonicalAthlete>>,
    ReportResult<Vec<CanonicalSchool>>,
    ReportResult<Vec<CanonicalTeam>>,
    ReportResult<Vec<CanonicalCoach>>,
    ReportResult<Vec<CanonicalCoach>>,
    ReportResult<Vec<CanonicalEvent>>,
    ReportResult<Vec<CanonicalMeet>>,
    ReportResult<Vec<CanonicalPerformance>>,
    ReportResult<Vec<ReviewCase>>,
    ReportResult<Vec<SourceAccessCondition>>,
    ReportResult<Vec<ReviewVerdictRecord>>,
    ReportResult<Vec<AppliedAthleteIdentity>>,
);

struct ReadTasks<'a> {
    athletes: ReadTask<'a, CanonicalAthlete>,
    schools: ReadTask<'a, CanonicalSchool>,
    teams: ReadTask<'a, CanonicalTeam>,
    coaches: ReadTask<'a, CanonicalCoach>,
    coach_observations: ReadTask<'a, CanonicalCoach>,
    events: ReadTask<'a, CanonicalEvent>,
    meets: ReadTask<'a, CanonicalMeet>,
    performances: ReadTask<'a, CanonicalPerformance>,
    review_cases: ReadTask<'a, ReviewCase>,
    source_access: ReadTask<'a, SourceAccessCondition>,
    verdicts: ReadTask<'a, ReviewVerdictRecord>,
    identity_decisions: ReadTask<'a, AppliedAthleteIdentity>,
}

impl<'a> ReadTasks<'a> {
    fn spawn<'env>(
        scope: &'a std::thread::Scope<'a, 'env>,
        snapshot: &'a StoreSnapshot<'_>,
    ) -> Self {
        Self {
            athletes: scope.spawn(|| Ok(snapshot.athletes()?)),
            schools: spawn_table(scope, snapshot, Table::Schools),
            teams: spawn_table(scope, snapshot, Table::Teams),
            coaches: spawn_table(scope, snapshot, Table::Coaches),
            coach_observations: scope.spawn(|| coach_observations(snapshot)),
            events: spawn_table(scope, snapshot, Table::Events),
            meets: spawn_table(scope, snapshot, Table::Meets),
            performances: spawn_table(scope, snapshot, Table::Performances),
            review_cases: spawn_table(scope, snapshot, Table::ReviewCases),
            source_access: spawn_table(scope, snapshot, Table::SourceAccess),
            verdicts: spawn_table(scope, snapshot, Table::IdentityVerdicts),
            identity_decisions: spawn_table(scope, snapshot, Table::AthleteIdentityDecisions),
        }
    }

    fn join(self) -> ReportResult<Loaded> {
        let joined = (
            join(self.athletes, "athletes"),
            join(self.schools, "schools"),
            join(self.teams, "teams"),
            join(self.coaches, "coaches"),
            join(self.coach_observations, "coach observations"),
            join(self.events, "events"),
            join(self.meets, "meets"),
            join(self.performances, "performances"),
            join(self.review_cases, "review cases"),
            join(self.source_access, "source access"),
            join(self.verdicts, "verdicts"),
            join(self.identity_decisions, "identity decisions"),
        );
        resolve(joined)
    }
}

fn resolve(joined: ReadResults) -> ReportResult<Loaded> {
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
}

fn spawn_table<'a, 'env, T: census_store::Entity + Send + 'a>(
    scope: &'a std::thread::Scope<'a, 'env>,
    snapshot: &'a StoreSnapshot<'_>,
    selected: Table,
) -> ReadTask<'a, T> {
    scope.spawn(move || table(snapshot, selected))
}
