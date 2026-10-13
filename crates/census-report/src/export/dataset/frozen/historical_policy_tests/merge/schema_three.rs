use super::super::super::fixtures::{seeded_store, SeededStore, TestResult};
use crate::report::CoverageReport;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalTeam, CoachLaneObligation,
    CoverageRow, CoverageScope, EventIdentity, EventKind, EventSpecification, ExactSeconds, Gender,
    Mark, PendingCoachLane, RelayMember, RelayResult, SchoolYear, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub(super) fn seed(policy: u32) -> TestResult<SeededStore> {
    let seeded = seeded_store()?;
    if policy == 15 {
        let row = CoverageRow::new(CoverageScope::Jurisdiction, "WI")
            .with("registered", 1)
            .with("captured", 1);
        check!(eq; CoverageReport::publish_coverage_rows(&seeded.store, &[row])?, 1);
        let lane = CoachLaneObligation::pending(PendingCoachLane {
            jurisdiction: UsJurisdiction::Wisconsin,
            season: SchoolYear::new(2026).ok_or("fixture season")?,
            revision: 1,
            arm: "policy15-retention-fixture".into(),
            arm_basis: "synthetic retained lane".into(),
            observed_on: "2026-05-01".into(),
        })?;
        seeded.store.replace(Table::CoachLanes, &lane)?;
        seed_relay(&seeded.store)?;
    }
    Ok(seeded)
}

fn seed_relay(store: &Store) -> TestResult {
    let meet = store
        .scan::<CanonicalMeet>(Table::Meets)?
        .into_iter()
        .next()
        .ok_or("fixture meet missing")?;
    let team = store
        .scan::<CanonicalTeam>(Table::Teams)?
        .into_iter()
        .next()
        .ok_or("fixture team missing")?;
    let athlete = store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .next()
        .ok_or("fixture athlete missing")?;
    let kind = EventKind::Relay4x100;
    let event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: kind.clone(),
            gender: Gender::Girls,
            division: None,
            round: None,
        },
        EventSpecification::from_published_label("4x100m Relay", &kind)?,
    )?;
    store.append(Table::Events, &event)?;
    let result = RelayResult::new(
        &team.id,
        &event.id,
        &meet.id,
        Mark::TimeSeconds(ExactSeconds::parse("44.25")?),
        "policy15-retained-relay",
    )
    .with_timing(Some(TimingMethod::Fat))
    .with_member(RelayMember::new(3, "Third published runner"))
    .with_member(RelayMember::new(1, "First published runner").with_athlete(athlete.id))
    .with_member(RelayMember::new(4, "Fourth published runner"))
    .with_member(RelayMember::new(2, "Second published runner"));
    store.append(Table::RelayResults, &result)?;
    Ok(())
}
