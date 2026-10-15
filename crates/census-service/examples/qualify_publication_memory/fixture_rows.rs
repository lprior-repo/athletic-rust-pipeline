use super::config::{Config, DAY};
use anyhow::{Context, Result};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub struct Owners {
    pub school: SchoolId,
    pub team: TeamId,
    pub meet: MeetId,
    pub event: EventId,
}

pub fn evidence(config: &Config, index: u64) -> Vec<Evidence> {
    (0..config.evidence_width)
        .map(|slot| {
            let mut fact = Evidence::parsed(
                SourceRef::new(
                    "synthetic_memory_fixture",
                    Some(format!("https://example.test/synthetic/{index}/{slot}")),
                ),
                DAY,
            );
            fact.note = Some("x".repeat(config.evidence_bytes));
            fact
        })
        .collect()
}

pub fn owners(store: &Store, state: UsJurisdiction) -> Result<Owners> {
    let (school, school_id) = CanonicalSchool::new(
        state,
        format!("Synthetic {} High", state.code()),
        format!("synthetic {} high", state.code()),
        None,
    );
    store.append(Table::Schools, &school)?;
    let team_id = Id::mint("team", &[school_id.as_str(), "synthetic-outdoor"]);
    let team = CanonicalTeam {
        id: team_id.clone(),
        school: school_id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Girls,
        school_year: SchoolYear::new(2025).context("fixture season")?,
        level: None,
        source_identities: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Teams, &team)?;
    let mut meet = CanonicalMeet::new_checked(
        Some(state),
        format!("Synthetic {} Invite", state.code()),
        DAY,
        CompetitionLevel::Invitational,
    )?;
    meet.sports.push(Sport::OutdoorTrack);
    store.append(Table::Meets, &meet)?;
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: Gender::Girls,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    let source = SourceRef::new(
        "synthetic_memory_fixture",
        Some("https://example.test/synthetic/event".into()),
    );
    event.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label: "400m".into(),
    });
    event.evidence.push(Evidence::parsed(source, DAY));
    store.append(Table::Events, &event)?;
    Ok(Owners {
        school: school_id,
        team: team_id,
        meet: meet.id,
        event: event.id,
    })
}

pub fn athlete(
    owners: &Owners,
    config: &Config,
    index: u64,
    name: String,
) -> Result<CanonicalAthlete> {
    let mut athlete = CanonicalAthlete::new(
        &owners.school,
        name,
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(
            SourceNamespace::MilesplitAthlete,
            index
                .checked_add(1)
                .context("synthetic provider ID overflow")?
                .to_string(),
        )
        .with_url(format!("https://example.test/synthetic/athlete/{index}")),
    )?;
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.evidence = evidence(config, index);
    let first = athlete.evidence.first().context("fixture evidence")?;
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: first.source.clone(),
    });
    Ok(athlete)
}

pub fn performance(
    owners: &Owners,
    athlete: &CanonicalAthlete,
    index: u64,
) -> Result<CanonicalPerformance> {
    let mut result = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            event: &owners.event,
            meet: &owners.meet,
            date: DAY,
            source_key: &format!("synthetic-result-{index}"),
        },
        PerformanceResult {
            team: &owners.team,
            mark: Mark::TimeSeconds(ExactSeconds::parse("60.00")?),
            wind_mps: None,
            adjustment: None,
            place: None,
        },
    )?;
    result.timing = Some(TimingMethod::Fat);
    result.source_athlete = athlete.source.clone();
    result.evidence = athlete.evidence.clone();
    Ok(result)
}
