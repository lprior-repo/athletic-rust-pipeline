use anyhow::{Context, Result};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub(super) fn seed(store: &Store) -> Result<()> {
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Policy fixture school",
        "policy fixture school",
        None,
    );
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Policy fixture runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "policy18-fixture-runner"),
    )?;
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: source(),
    });
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    for (label, sport) in [
        ("300m", Sport::OutdoorTrack),
        ("500m", Sport::OutdoorTrack),
        ("600m", Sport::IndoorTrack),
        ("2 miles", Sport::OutdoorTrack),
        ("3200m", Sport::OutdoorTrack),
        ("3000m", Sport::OutdoorTrack),
        ("300m Hurdles", Sport::OutdoorTrack),
        ("Cross Country 2 miles", Sport::CrossCountry),
        ("1000m (300m banked)", Sport::IndoorTrack),
        ("400m banked track", Sport::IndoorTrack),
        ("300m (200m banked)", Sport::IndoorTrack),
    ] {
        let meet = meet(label, sport);
        let team = team(&school_id, sport)?;
        let event = event(&meet, label)?;
        let performance = performance(&athlete, &team, &event)?;
        store.append(Table::Teams, &team)?;
        store.append(Table::Meets, &meet)?;
        store.append(Table::Events, &event)?;
        store.append(Table::Performances, &performance)?;
    }
    Ok(())
}

fn source() -> SourceRef {
    SourceRef::new(
        "policy18_fixture",
        Some("https://example.invalid/policy18/results".into()),
    )
}

fn meet(label: &str, sport: Sport) -> CanonicalMeet {
    CanonicalMeet {
        id: Id::mint("meet", &["policy18-fixture", label]),
        name: label.into(),
        normalized_name: label.into(),
        date: "2025-03-01".into(),
        end_date: None,
        location: None,
        state: Some(UsJurisdiction::Wisconsin),
        level: CompetitionLevel::Invitational,
        sports: vec![sport],
        source_identities: Vec::new(),
        source_urls: Vec::new(),
        evidence: vec![Evidence::parsed(source(), "2025-03-01")],
        retained_conflicts: Vec::new(),
    }
}

fn team(school: &SchoolId, sport: Sport) -> Result<CanonicalTeam> {
    let school_year = SchoolYear::new(2024).context("fixture school year")?;
    Ok(CanonicalTeam {
        id: CanonicalTeam::mint(school, sport, Gender::Boys, school_year),
        school: school.clone(),
        sport,
        gender: Gender::Boys,
        school_year,
        level: None,
        source_identities: vec![SourceIdentity::new(
            SourceNamespace::MilesplitTeam,
            "policy18-fixture-team",
        )],
        evidence: vec![Evidence::parsed(source(), "2025-03-01")],
        retained_conflicts: Vec::new(),
    })
}

fn event(meet: &CanonicalMeet, label: &str) -> Result<CanonicalEvent> {
    let kind = EventKind::from_source_label(label);
    let mut specification = EventSpecification::from_published_label(label, &kind)?;
    if let Some(course) = specification.cross_country.as_mut() {
        course.course = Some(CourseIdentity::new("policy18-fixture-course", "1")?);
        course.measurement = CourseMeasurement::PublishedMeasured;
    }
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        specification,
    )?;
    event.source_labels.push(SourceEventLabel {
        source: source(),
        label: label.into(),
    });
    event
        .evidence
        .push(Evidence::parsed(source(), "2025-03-01"));
    Ok(event)
}

fn performance(
    athlete: &CanonicalAthlete,
    team: &CanonicalTeam,
    event: &CanonicalEvent,
) -> Result<CanonicalPerformance> {
    let mut performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            meet: &event.meet,
            event: &event.id,
            date: "2025-03-01",
            source_key: event.id.as_str(),
        },
        PerformanceResult {
            team: &team.id,
            mark: Mark::TimeSeconds(ExactSeconds::parse("60.10")?),
            adjustment: None,
            wind_mps: None,
            place: Some(1),
        },
    )?;
    performance.source_athlete = athlete.source.clone();
    performance.timing = Some(TimingMethod::Fat);
    performance
        .evidence
        .push(Evidence::parsed(source(), "2025-03-01"));
    Ok(performance)
}
