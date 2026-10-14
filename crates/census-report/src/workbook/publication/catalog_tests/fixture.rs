use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) struct Fixture {
    pub directory: tempfile::TempDir,
    pub store: Store,
    pub athlete: AthleteId,
    pub ordinary: EventId,
    pub excluded: EventId,
    pub unresolved: EventId,
    pub relay: RelayId,
}

pub(super) fn fixture() -> TestResult<Fixture> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Catalog School",
        "catalog school",
        None,
    );
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Catalog runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "catalog_runner"),
    )?;
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: source("catalog_roster"),
    });
    let meet = meet();
    let season = SchoolYear::from_date(&meet.date).ok_or("fixture season")?;
    let owner = SourceIdentity::new(SourceNamespace::MilesplitTeam, "catalog_team");
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(&school_id, Sport::OutdoorTrack, Gender::Boys, season),
        school: school_id,
        sport: Sport::OutdoorTrack,
        gender: Gender::Boys,
        school_year: season,
        level: None,
        source_identities: vec![owner.clone()],
        evidence: vec![Evidence::parsed(source("wiaa_results"), &meet.date)],
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    store.append(Table::Teams, &team)?;
    store.append(Table::Meets, &meet)?;
    let ordinary = event(&meet, EventKind::Track100m, "100m")?;
    let excluded = event(
        &meet,
        EventKind::Unmapped {
            label: "Wheelchair 100m".into(),
        },
        "Wheelchair 100m",
    )?;
    let walking = event(
        &meet,
        EventKind::Unmapped {
            label: "Racewalk 100m".into(),
        },
        "Racewalk 100m",
    )?;
    let unresolved = event(
        &meet,
        EventKind::Unmapped {
            label: "650m".into(),
        },
        "650m",
    )?;
    for event in [&ordinary, &excluded, &walking, &unresolved] {
        store.append(Table::Events, event)?;
        let performance = performance(&athlete, &team.id, event)?;
        store.append(Table::Performances, &performance)?;
    }
    let included_relay = event(&meet, EventKind::Relay4x100, "4x100m")?;
    let excluded_relay = event(
        &meet,
        EventKind::Unmapped {
            label: "Wheelchair 4x100m".into(),
        },
        "Wheelchair 4x100m",
    )?;
    let included_result = relay(&athlete, &team.id, &included_relay, &owner)?;
    let excluded_result = relay(&athlete, &team.id, &excluded_relay, &owner)?;
    for (event, result) in [
        (&included_relay, &included_result),
        (&excluded_relay, &excluded_result),
    ] {
        store.append(Table::Events, event)?;
        store.append(Table::RelayResults, result)?;
    }
    Ok(Fixture {
        directory,
        store,
        athlete: athlete.id,
        ordinary: ordinary.id,
        excluded: excluded.id,
        unresolved: unresolved.id,
        relay: included_result.id,
    })
}

fn source(id: &str) -> SourceRef {
    SourceRef::new(id, Some(format!("https://example.test/{id}/catalog")))
}

fn meet() -> CanonicalMeet {
    CanonicalMeet {
        id: Id::mint("meet", &["catalog-whole-meet"]),
        name: "Catalog whole meet".into(),
        normalized_name: "catalog whole meet".into(),
        date: "2025-03-01".into(),
        end_date: None,
        location: None,
        state: Some(UsJurisdiction::Wisconsin),
        level: CompetitionLevel::Unknown,
        sports: vec![Sport::OutdoorTrack],
        source_identities: Vec::new(),
        source_urls: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    }
}

fn event(meet: &CanonicalMeet, kind: EventKind, label: &str) -> TestResult<CanonicalEvent> {
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    let source = source("wiaa_results");
    event.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label: label.into(),
    });
    event.evidence.push(Evidence::parsed(source, &meet.date));
    Ok(event)
}

fn performance(
    athlete: &CanonicalAthlete,
    team: &TeamId,
    event: &CanonicalEvent,
) -> TestResult<CanonicalPerformance> {
    let mut result = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            meet: &event.meet,
            event: &event.id,
            date: "2025-03-01",
            source_key: event.id.as_str(),
        },
        PerformanceResult {
            team,
            mark: Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
            adjustment: None,
            wind_mps: Some(1.0),
            place: Some(1),
        },
    )?;
    result.source_athlete = athlete.source.clone();
    result.timing = Some(TimingMethod::Fat);
    result
        .evidence
        .push(Evidence::parsed(source("wiaa_results"), "2025-03-01"));
    Ok(result)
}

fn relay(
    athlete: &CanonicalAthlete,
    team: &TeamId,
    event: &CanonicalEvent,
    owner: &SourceIdentity,
) -> TestResult<RelayResult> {
    Ok(RelayResult::new(
        team,
        &event.id,
        &event.meet,
        Mark::TimeSeconds(ExactSeconds::parse("44.25")?),
        event.id.as_str(),
    )
    .with_source_team(owner.clone())
    .with_timing(Some(TimingMethod::Fat))
    .with_member(RelayMember::new(3, "Catalog runner").with_athlete(athlete.id.clone()))
    .with_member(RelayMember::new(1, "Second published runner"))
    .with_member(RelayMember::new(4, "Third published runner"))
    .with_member(RelayMember::new(2, "Fourth published runner"))
    .with_evidence(Evidence::parsed(source("wiaa_results"), "2025-03-01")))
}
