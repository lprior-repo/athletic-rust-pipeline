use super::*;
use census_domain::model::{
    CompetitionLevel, EventKind, EventSpecification, Gender, Id, Mark, SchoolYear, Sport,
};

pub(super) fn append(fixture: &Fixture) -> TestResult {
    let athlete = fixture
        .store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .next()
        .ok_or("athlete")?;
    fixture.store.append(Table::Athletes, &athlete)?;
    fixture::accept(&fixture.store, &fixture.writer, &fixture.athlete)?;
    let mut team = CanonicalTeam {
        id: Id::mint("team", &["raw-proof"]),
        school: fixture.school.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Girls,
        school_year: SchoolYear::DEFAULT,
        level: None,
        source_identities: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    fixture.store.append(Table::Teams, &team)?;
    team.sport = Sport::CrossCountry;
    fixture.store.append(Table::Teams, &team)?;
    let mut meet = CanonicalMeet {
        id: Id::mint("meet", &["raw-proof"]),
        name: "Original meet".into(),
        normalized_name: "original-meet".into(),
        date: "2026-10-07".into(),
        end_date: None,
        location: None,
        state: None,
        level: CompetitionLevel::Unknown,
        sports: vec![Sport::OutdoorTrack],
        source_identities: Vec::new(),
        source_urls: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    fixture.store.append(Table::Meets, &meet)?;
    meet.name = "Later meet".into();
    fixture.store.append(Table::Meets, &meet)?;
    let mut event = CanonicalEvent {
        id: Id::mint("evt", &["raw-proof"]),
        meet: meet.id.clone(),
        kind: EventKind::Track100m,
        gender: Gender::Girls,
        specification: EventSpecification::default(),
        division: None,
        round: None,
        source_labels: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    fixture.store.append(Table::Events, &event)?;
    event.division = Some("later division".into());
    fixture.store.append(Table::Events, &event)?;
    let mut performance = CanonicalPerformance {
        id: Id::mint("perf", &["raw-proof"]),
        athlete: fixture.athlete.clone(),
        team: team.id,
        event: event.id,
        meet: meet.id,
        date: "2026-10-07".into(),
        mark: Mark::Raw("original".into()),
        adjustment: None,
        wind_mps: None,
        place: None,
        heat: None,
        round: None,
        timing: None,
        observed_grade: None,
        evidence: Vec::new(),
        source_key: "original".into(),
        source_athlete: None,
        retained_conflicts: Vec::new(),
        combined_completion: None,
    };
    fixture.store.append(Table::Performances, &performance)?;
    performance.source_key = "later".into();
    fixture.store.append(Table::Performances, &performance)?;
    Ok(())
}
