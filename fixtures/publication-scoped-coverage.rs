use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution,
    CoachLaneStage, CoachRole, CompetitionLevel, CoverageRow, CoverageScope, EventIdentity,
    EventKind, EventSpecification, Evidence, ExactSeconds, Gender, GradYear, Mark,
    PendingCoachLane, PerformanceIdentity, PerformanceResult, PublishedGraduation, SchoolYear,
    SourceEventLabel, SourceIdentity, SourceNamespace, SourceRef, Sport, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn seed(store: &Store) -> TestResult {
    let wi = school(store, UsJurisdiction::Wisconsin, "Scoped Wisconsin", false)?;
    let mn = school(
        store,
        UsJurisdiction::Minnesota,
        "Historical Minnesota",
        false,
    )?;
    let unknown = school(store, UsJurisdiction::Iowa, "Unplaced High", true)?;
    let wi_athlete = athlete(store, &wi, "Wisconsin Runner", "wiaa_results", Some(2027))?;
    let mn_athlete = athlete(store, &mn, "Minnesota Runner", "mshsl_results", Some(2027))?;
    let unknown_athlete = athlete(
        store,
        &unknown,
        "Unplaced Runner",
        "wiaa_results",
        Some(2027),
    )?;
    athlete(store, &wi, "Unsupported Runner", "wiaa_results", None)?;
    athlete(store, &wi, "Later Cohort", "wiaa_results", Some(2028))?;
    let non_core = athlete(
        store,
        &wi,
        "Noncore Runner",
        "athleticlive_athletes",
        Some(2027),
    )?;
    let wi_meet = meet(store, UsJurisdiction::Wisconsin, "Wisconsin Meet")?;
    let mn_meet = meet(
        store,
        UsJurisdiction::Minnesota,
        "Historical Minnesota Meet",
    )?;
    result(
        store,
        &wi_athlete,
        &mn,
        &mn_meet,
        "wi-historical",
        "wiaa_results",
    )?;
    result(
        store,
        &mn_athlete,
        &mn,
        &mn_meet,
        "mn-local",
        "mshsl_results",
    )?;
    result(
        store,
        &unknown_athlete,
        &unknown,
        &wi_meet,
        "unknown-local",
        "wiaa_results",
    )?;
    result(
        store,
        &non_core,
        &wi,
        &wi_meet,
        "noncore-local",
        "athleticlive_athletes",
    )?;
    store.replace(Table::Coverage, &coverage(UsJurisdiction::Wisconsin, 7))?;
    store.replace(Table::Coverage, &coverage(UsJurisdiction::Minnesota, 9))?;
    store.replace(Table::CoachLanes, &lane(UsJurisdiction::Wisconsin)?)?;
    let blocked = lane(UsJurisdiction::Minnesota)?.record(
        CoachLaneStage::Blocked,
        Some(CoachLaneRefusal::new(
            SourceRef::id("synthetic_scoped_directory"),
            "retained outside-jurisdiction refusal",
            "2026-05-01",
        )?),
        CoachLaneResolution::unresolved("2026-05-01"),
    )?;
    store.replace(Table::CoachLanes, &blocked)?;
    Ok(())
}

fn school(
    store: &Store,
    state: UsJurisdiction,
    name: &str,
    unplaced: bool,
) -> TestResult<CanonicalSchool> {
    let (mut school, _) = CanonicalSchool::new(state, name, name.to_lowercase(), None);
    if unplaced {
        school.state = None;
    }
    school.evidence.push(evidence("wiaa_results"));
    store.append(Table::Schools, &school)?;
    let mut coach = CanonicalCoach::new(
        &school.id,
        format!("{name} Coach"),
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    coach.evidence.push(evidence("wiaa_results"));
    store.append(Table::Coaches, &coach)?;
    Ok(school)
}

fn athlete(
    store: &Store,
    school: &CanonicalSchool,
    name: &str,
    source: &str,
    year: Option<i16>,
) -> TestResult<CanonicalAthlete> {
    let grad_year =
        GradYear::new(year.map_or(2027, core::convert::identity)).ok_or("fixture cohort")?;
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        name,
        grad_year,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, name),
    );
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.evidence.push(evidence(source));
    if year.is_some() {
        athlete.published_graduations.push(PublishedGraduation {
            grad_year,
            source: SourceRef::new(source, Some("https://example.test/scoped-coverage".into())),
        });
    }
    store.append(Table::Athletes, &athlete)?;
    Ok(athlete)
}

pub(super) fn meet(store: &Store, state: UsJurisdiction, name: &str) -> TestResult<CanonicalMeet> {
    let mut meet = CanonicalMeet::new_checked(
        Some(state),
        name,
        "2026-05-01",
        CompetitionLevel::Invitational,
    )?;
    meet.sports.push(Sport::OutdoorTrack);
    meet.evidence.push(evidence("wiaa_results"));
    store.append(Table::Meets, &meet)?;
    Ok(meet)
}

pub(super) fn result(
    store: &Store,
    athlete: &CanonicalAthlete,
    historical_school: &CanonicalSchool,
    meet: &CanonicalMeet,
    key: &str,
    source: &str,
) -> TestResult {
    let season = SchoolYear::new(2026).ok_or("fixture season")?;
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(
            &historical_school.id,
            Sport::OutdoorTrack,
            Gender::Girls,
            season,
        ),
        school: historical_school.id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Girls,
        school_year: season,
        level: None,
        source_identities: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Teams, &team)?;
    let (kind, label, seconds) = if key == "mn-local" {
        (EventKind::Track800m, "800m", "121.00")
    } else {
        (EventKind::Track400m, "400m", "50.00")
    };
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind,
            gender: Gender::Girls,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    event.source_labels.push(SourceEventLabel {
        source: evidence(source).source,
        label: label.into(),
    });
    event.evidence.push(evidence(source));
    store.append(Table::Events, &event)?;
    let mut performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            event: &event.id,
            meet: &meet.id,
            date: "2026-05-01",
            source_key: key,
        },
        PerformanceResult {
            team: &team.id,
            mark: Mark::TimeSeconds(ExactSeconds::parse(seconds)?),
            wind_mps: None,
            adjustment: None,
            place: Some(1),
        },
    )?;
    performance.source_athlete = athlete.source.clone();
    performance.timing = Some(TimingMethod::Fat);
    performance.evidence.push(evidence(source));
    store.append(Table::Performances, &performance)?;
    Ok(())
}

fn evidence(source: &str) -> Evidence {
    Evidence::parsed(
        SourceRef::new(source, Some("https://example.test/scoped-coverage".into())),
        "2026-05-01",
    )
}

fn coverage(jurisdiction: UsJurisdiction, registered: u64) -> CoverageRow {
    CoverageRow::new(CoverageScope::Jurisdiction, jurisdiction.code())
        .with("registered", registered)
        .with("applicable", registered)
        .with("attempted", 6)
        .with("captured", 5)
        .with("parsed", 4)
        .with("accepted", 3)
        .with("withheld", 2)
}

fn lane(jurisdiction: UsJurisdiction) -> TestResult<CoachLaneObligation> {
    Ok(CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction,
        season: SchoolYear::new(2026).ok_or("fixture season")?,
        revision: 1,
        arm: "synthetic_scoped_directory".into(),
        arm_basis: "isolated scoped publication fixture".into(),
        observed_on: "2026-05-01".into(),
    })?)
}
