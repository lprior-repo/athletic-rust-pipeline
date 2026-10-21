use super::super::TestResult;
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub(super) fn historical_performance(
    store: &Store,
    athlete: &CanonicalAthlete,
) -> TestResult<CanonicalPerformance> {
    let source = SourceRef::new(
        "fixture-results",
        Some("https://results.test/meet/1001".to_owned()),
    );
    let evidence = Evidence::parsed(source, "2026-06-01");
    let meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Prior School Invitational",
        "2026-06-01",
        CompetitionLevel::State,
    );
    let team = team(athlete, &evidence)?;
    let event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: Gender::Girls,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    let performance = performance(athlete, &meet, &team, &event, evidence)?;
    store.append(Table::Meets, &meet)?;
    store.append(Table::Teams, &team)?;
    store.append(Table::Events, &event)?;
    store.append(Table::Performances, &performance)?;
    Ok(performance)
}

fn team(athlete: &CanonicalAthlete, evidence: &Evidence) -> TestResult<CanonicalTeam> {
    let year = SchoolYear::new(2025).ok_or("missing fixture year")?;
    Ok(CanonicalTeam {
        id: CanonicalTeam::mint(&athlete.school, Sport::OutdoorTrack, Gender::Girls, year),
        school: athlete.school.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Girls,
        school_year: year,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![evidence.clone()],
        retained_conflicts: Vec::new(),
    })
}

fn performance(
    athlete: &CanonicalAthlete,
    meet: &CanonicalMeet,
    team: &CanonicalTeam,
    event: &CanonicalEvent,
    evidence: Evidence,
) -> TestResult<CanonicalPerformance> {
    Ok(CanonicalPerformance {
        id: CanonicalPerformance::mint(
            &athlete.id,
            &meet.id,
            &event.id,
            &meet.date,
            "61k:original-result",
        ),
        athlete: athlete.id.clone(),
        team: team.id.clone(),
        event: event.id.clone(),
        meet: meet.id.clone(),
        date: meet.date.clone(),
        mark: Mark::TimeSeconds(ExactSeconds::parse("61.50")?),
        adjustment: None,
        wind_mps: None,
        place: None,
        heat: None,
        round: None,
        timing: Some(TimingMethod::Fat),
        observed_grade: None,
        evidence: vec![evidence],
        source_key: "61k:original-result".to_owned(),
        source_athlete: athlete.source.clone(),
        retained_conflicts: Vec::new(),
        combined_completion: None,
    })
}
