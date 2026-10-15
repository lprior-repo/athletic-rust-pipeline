use super::{ReviewedFixture, TestResult};
use crate::{Entity, Store, Table};
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalPerformance, EventId, Mark, MeetId, PerformanceIdentity,
    PerformanceResult, RelayMember, RelayResult, SourceAthleteObservation, SourceNamespace,
    SourceObservation, TeamId,
};

pub(super) fn install(fixture: &ReviewedFixture) -> TestResult {
    let (performances, relays) = rows(fixture)?;
    fixture
        .store
        .append_many(Table::Performances, &performances)?;
    fixture.store.append_many(Table::RelayResults, &relays)?;
    fixture
        .store
        .append(Table::SourceObservations, &unrelated_observation())?;
    Ok(())
}

pub(super) fn rows(
    fixture: &ReviewedFixture,
) -> TestResult<(Vec<CanonicalPerformance>, Vec<RelayResult>)> {
    let [first, second, unrelated] = &fixture.candidates;
    let first = first.cast();
    let second = second.cast();
    let unrelated = unrelated.cast();
    let mut conflicting_performance = performance(&unrelated, "raw-variant")?;
    let mut member_performance = conflicting_performance.clone();
    member_performance.athlete = first.clone();
    member_performance.mark = Mark::Raw("12.20".into());
    conflicting_performance.mark = Mark::Raw("12.21".into());
    let conflicting_relay = relay(&unrelated, "raw-variant");
    let mut member_relay = conflicting_relay.clone();
    member_relay.members =
        vec![RelayMember::new(1, "Member synthetic runner").with_athlete(second.clone())];
    Ok((
        vec![
            performance(&first, "first-member")?,
            performance(&second, "second-member")?,
            performance(&unrelated, "unrelated")?,
            conflicting_performance,
            member_performance,
        ],
        vec![
            relay(&first, "first-member"),
            relay(&second, "second-member"),
            relay(&unrelated, "unrelated"),
            conflicting_relay,
            member_relay,
        ],
    ))
}

pub(super) fn expected_keys(fixture: &ReviewedFixture) -> TestResult<Vec<String>> {
    let [first, second, unrelated] = &fixture.candidates;
    let inputs = super::super::Inputs::capture(&fixture.store.snapshot())?;
    let observations: Vec<_> = inputs
        .observations
        .iter()
        .filter(|row| row.id() != unrelated_observation().id())
        .collect();
    assert_eq!(observations.len(), 1);
    let mut keys = vec![format!("athletes:{first}"), format!("athletes:{second}")];
    keys.extend(
        observations
            .iter()
            .map(|row| format!("source_observations:{}", row.id())),
    );
    for (athlete, label) in [
        (first, "first-member"),
        (second, "second-member"),
        (unrelated, "raw-variant"),
    ] {
        keys.push(format!(
            "performances:{}",
            performance(&athlete.cast(), label)?.id
        ));
        keys.push(format!(
            "relay_results:{}",
            relay(&athlete.cast(), label).id
        ));
    }
    keys.sort();
    Ok(keys)
}

pub(super) fn unrelated_keys(fixture: &ReviewedFixture) -> TestResult<Vec<String>> {
    let unrelated = fixture.candidates.last().ok_or("unrelated candidate")?;
    Ok(vec![
        format!("athletes:{unrelated}"),
        format!("source_observations:{}", unrelated_observation().id()),
        format!(
            "performances:{}",
            performance(&unrelated.cast(), "unrelated")?.id
        ),
        format!("relay_results:{}", relay(&unrelated.cast(), "unrelated").id),
    ])
}

pub(super) fn performance(athlete: &AthleteId, label: &str) -> TestResult<CanonicalPerformance> {
    let meet = MeetId::mint("meet", &["manual-synthetic", label]);
    let event = EventId::mint("event", &["manual-synthetic-100m", label]);
    let team = TeamId::mint("team", &["manual-synthetic-team", label]);
    Ok(CanonicalPerformance::new(
        PerformanceIdentity {
            athlete,
            event: &event,
            meet: &meet,
            date: "2026-10-14",
            source_key: label,
        },
        PerformanceResult {
            team: &team,
            mark: Mark::Raw("12.19".into()),
            wind_mps: None,
            place: Some(1),
            adjustment: None,
        },
    )?)
}

pub(super) fn relay(athlete: &AthleteId, label: &str) -> RelayResult {
    RelayResult::new(
        &TeamId::mint("team", &["manual-synthetic-relay-team", label]),
        &EventId::mint("event", &["manual-synthetic-4x100", label]),
        &MeetId::mint("meet", &["manual-synthetic", label]),
        Mark::Raw("49.19".into()),
        label,
    )
    .with_member(RelayMember::new(1, "Synthetic runner").with_athlete(athlete.clone()))
}

fn unrelated_observation() -> SourceObservation {
    SourceObservation::Athlete(SourceAthleteObservation::new(
        SourceNamespace::MilesplitAthlete,
        "9204",
        "synthetic-unrelated-source-row",
        "Synthetic Runner reviewed-unrelated",
        "2026-10-14",
    ))
}

pub(super) fn raw<T: Entity>(store: &Store, table: Table) -> TestResult<Vec<T>> {
    let mut rows = Vec::new();
    store.snapshot().for_each_observation(table, |row| {
        rows.push(row);
        Ok(())
    })?;
    Ok(rows)
}

pub(super) fn retained_facts(store: &Store) -> TestResult<serde_json::Value> {
    Ok(serde_json::to_value((
        raw::<CanonicalAthlete>(store, Table::Athletes)?,
        raw::<SourceObservation>(store, Table::SourceObservations)?,
        raw::<CanonicalPerformance>(store, Table::Performances)?,
        raw::<RelayResult>(store, Table::RelayResults)?,
    ))?)
}
