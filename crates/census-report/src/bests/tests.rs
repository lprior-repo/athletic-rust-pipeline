use crate::bests::{build_from_dataset, Options, SharedSelection};
use crate::export::ExportDataset;
use crate::report::Scope;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CentiMetres, CentiPoints, CentiSeconds, EventId, EventKind, Gender, GradYear,
    Id, Mark, MeetId, PerformanceId, SourceIdentity, SourceNamespace, Sport, TeamId, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod classification;
mod conflicts;
mod keys;
mod measures;
mod reduction;
mod units;

fn athlete_id() -> AthleteId {
    Id::mint("ath", &["test_athlete"])
}

fn meet_id() -> MeetId {
    Id::mint("meet", &["test_meet"])
}

fn event_id() -> EventId {
    Id::mint("evt", &["test_evt"])
}

fn team_id() -> TeamId {
    Id::mint("team", &["test_team"])
}

fn performance_id() -> PerformanceId {
    Id::mint("perf", &["test_perf"])
}

fn test_meet(sports: Vec<Sport>) -> CanonicalMeet {
    CanonicalMeet {
        id: meet_id(),
        name: "Test Meet".to_string(),
        normalized_name: "test_meet".to_string(),
        date: "2025-03-01".to_string(),
        end_date: None,
        location: None,
        state: Some(UsJurisdiction::Wisconsin),
        level: census_domain::model::CompetitionLevel::Unknown,
        sports,
        source_identities: vec![],
        source_urls: vec![],
        evidence: vec![],
        retained_conflicts: vec![],
    }
}

fn test_performance(
    _kind: EventKind,
    mark: Mark,
    timing: Option<TimingMethod>,
    wind_mps: Option<f64>,
    date: &str,
) -> CanonicalPerformance {
    CanonicalPerformance {
        id: performance_id(),
        athlete: athlete_id(),
        team: team_id(),
        event: event_id(),
        meet: meet_id(),
        date: date.to_string(),
        mark,
        wind_mps,
        place: None,
        heat: None,
        round: None,
        timing,
        observed_grade: None,
        evidence: vec![],
        source_key: "test_key".to_string(),
        source_athlete: Some(SourceIdentity::new(
            census_domain::model::SourceNamespace::MilesplitAthlete,
            "test_athlete_id",
        )),
        retained_conflicts: vec![],
    }
}

fn test_parent_meet(
    sports: Vec<Sport>,
    state: Option<UsJurisdiction>,
    name: &str,
) -> CanonicalMeet {
    CanonicalMeet {
        id: meet_id(),
        name: name.to_string(),
        normalized_name: name.to_lowercase().replace(' ', "_"),
        date: "2025-03-01".to_string(),
        end_date: None,
        location: None,
        state,
        level: census_domain::model::CompetitionLevel::Unknown,
        sports,
        source_identities: vec![],
        source_urls: vec![],
        evidence: vec![],
        retained_conflicts: vec![],
    }
}

fn test_parent_performance(
    _kind: EventKind,
    mark: Mark,
    timing: Option<TimingMethod>,
    wind_mps: Option<f64>,
    date: &str,
    meet: Option<&CanonicalMeet>,
) -> CanonicalPerformance {
    CanonicalPerformance {
        id: performance_id(),
        athlete: athlete_id(),
        team: team_id(),
        event: event_id(),
        meet: meet
            .map(|m| m.id.clone())
            .map_or(meet_id(), std::convert::identity),
        date: date.to_string(),
        mark,
        wind_mps,
        place: None,
        heat: None,
        round: None,
        timing,
        observed_grade: None,
        evidence: vec![],
        source_key: "test_key".to_string(),
        source_athlete: Some(SourceIdentity::new(
            census_domain::model::SourceNamespace::MilesplitAthlete,
            "test_athlete_id",
        )),
        retained_conflicts: vec![],
    }
}

fn selection_dataset(kind: EventKind) -> TestResult<ExportDataset> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Synthetic school",
        "synthetic school",
        None,
    );
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Synthetic runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001"),
    );
    athlete
        .published_graduations
        .push(census_domain::model::PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: census_domain::model::SourceRef::new(
                "synthetic_published_roster",
                Some("https://example.invalid/roster/1001".to_string()),
            ),
        });
    let meet = test_meet(vec![Sport::OutdoorTrack]);
    let event = CanonicalEvent::new(&meet.id, kind, Gender::Boys, None, None);
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    store.append(Table::Meets, &meet)?;
    store.append(Table::Events, &event)?;
    Ok(ExportDataset::load(&store)?)
}

fn reported(dataset: &ExportDataset, label: &str, mark: Mark) -> CanonicalPerformance {
    let athlete = &dataset.athletes[0];
    let event = &dataset.events[0];
    let meet = &dataset.meets[0];
    let mut performance = test_performance(
        event.kind.clone(),
        mark,
        Some(TimingMethod::Fat),
        Some(1.0),
        &meet.date,
    );
    performance.id = Id::mint("perf", &[label]);
    performance.athlete = athlete.id.clone();
    performance.meet = meet.id.clone();
    performance.event = event.id.clone();
    performance.source_athlete = athlete.source.clone();
    performance.source_key = label.into();
    performance
}

fn selected(dataset: &ExportDataset) -> Vec<SharedSelection> {
    build_from_dataset(
        dataset,
        &Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
        },
    )
}
