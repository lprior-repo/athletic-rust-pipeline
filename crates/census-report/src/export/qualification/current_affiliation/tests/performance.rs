use super::*;
use census_domain::UsJurisdiction;

fn evidence() -> Evidence {
    Evidence::parsed(
        SourceRef::new(
            "synthetic historical result",
            Some("https://result-fixture.test/result/old-result".into()),
        ),
        "2026-05-01",
    )
}

pub(super) fn append_performance(store: &Store, athlete: &CanonicalAthlete) -> TestResult {
    let season = SchoolYear::new(2025).ok_or("fixture season")?;
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(&athlete.school, Sport::OutdoorTrack, Gender::Girls, season),
        school: athlete.school.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Girls,
        school_year: season,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![evidence()],
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Teams, &team)?;
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Old school meet",
        "2026-05-01",
        CompetitionLevel::Invitational,
    );
    meet.evidence = vec![evidence()];
    store.append(Table::Meets, &meet)?;
    let mut event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: Gender::Girls,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?);
    event.evidence = vec![evidence()];
    store.append(Table::Events, &event)?;
    let performance = CanonicalPerformance {
        id: CanonicalPerformance::mint(
            &athlete.id,
            &meet.id,
            &event.id,
            "2026-05-01",
            "old-result",
        ),
        athlete: athlete.id.clone(),
        team: team.id,
        event: event.id,
        meet: meet.id,
        date: "2026-05-01".into(),
        mark: Mark::TimeSeconds(ExactSeconds::parse("60.00")?),
        adjustment: None,
        wind_mps: None,
        place: None,
        heat: None,
        round: None,
        timing: None,
        observed_grade: None,
        evidence: vec![evidence()],
        source_key: "old-result".into(),
        source_athlete: None,
        retained_conflicts: Vec::new(),
        combined_completion: None,
    };
    store.append(Table::Performances, &performance)?;
    Ok(())
}
