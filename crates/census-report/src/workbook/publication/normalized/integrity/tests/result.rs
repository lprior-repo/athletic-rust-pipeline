use crate::export::public_identity::tests::fixture::TestResult;
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub(super) fn append(
    store: &Store,
    athlete: &CanonicalAthlete,
    label: &str,
    date: &str,
) -> TestResult {
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(
            &athlete.school,
            Sport::OutdoorTrack,
            athlete.gender,
            SchoolYear::DEFAULT,
        ),
        school: athlete.school.clone(),
        sport: Sport::OutdoorTrack,
        gender: athlete.gender,
        school_year: SchoolYear::DEFAULT,
        level: None,
        source_identities: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    let mut meet = CanonicalMeet::new_checked(
        Some(UsJurisdiction::Michigan),
        format!("Synthetic identity join meet {label}"),
        date,
        CompetitionLevel::Invitational,
    )?;
    meet.sports.push(Sport::OutdoorTrack);
    let event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: athlete.gender,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?);
    let source_key = format!("synthetic-identity-join-{label}");
    let mut performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            event: &event.id,
            meet: &meet.id,
            date,
            source_key: &source_key,
        },
        PerformanceResult {
            team: &team.id,
            mark: Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
            wind_mps: None,
            adjustment: None,
            place: Some(1),
        },
    )?;
    performance.timing = Some(TimingMethod::Fat);
    performance.source_athlete = athlete.source.clone();
    performance.evidence.push(Evidence::parsed(
        SourceRef::new(
            "synthetic-identity-results",
            Some(format!("https://synthetic-identity.test/results/{label}")),
        ),
        date,
    ));
    store.append(Table::Teams, &team)?;
    store.append(Table::Meets, &meet)?;
    store.append(Table::Events, &event)?;
    store.append(Table::Performances, &performance)?;
    Ok(())
}
