use super::*;
use crate::public_identity::{context, local_adjudication};
use census_domain::model::{
    AthleteId, CanonicalPerformance, EventId, Mark, MeetId, PerformanceIdentity, PerformanceResult,
    RelayMember, RelayResult, TeamId,
};

fn performance(athlete: &AthleteId, label: &str) -> TestResult<CanonicalPerformance> {
    let meet = MeetId::mint("meet", &["graph-affected", label]);
    let event = EventId::mint("event", &["graph-100m", label]);
    let team = TeamId::mint("team", &["graph-team", label]);
    Ok(CanonicalPerformance::new(
        PerformanceIdentity {
            athlete,
            event: &event,
            meet: &meet,
            date: "2026-10-15",
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

fn relay(athlete: &AthleteId, label: &str) -> RelayResult {
    RelayResult::new(
        &TeamId::mint("team", &["graph-relay-team", label]),
        &EventId::mint("event", &["graph-4x100", label]),
        &MeetId::mint("meet", &["graph-relay", label]),
        Mark::Raw("49.19".into()),
        label,
    )
    .with_member(RelayMember::new(1, "Runner").with_athlete(athlete.clone()))
}

#[test]
fn later_page_physical_performance_and_relay_variants_match_historical_keys() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    let target = subject("affected-target", "63001")?;
    let other = subject("affected-other", "63002")?;
    store.append_many(Table::Athletes, &[target.clone(), other.clone()])?;
    let mut performances = Vec::new();
    let mut relays = Vec::new();
    for index in 0..520 {
        performances.push(performance(&other.id, &format!("page-{index}"))?);
        relays.push(relay(&other.id, &format!("page-{index}")));
    }
    performances.sort_by(|left, right| left.id.cmp(&right.id));
    relays.sort_by(|left, right| left.id.cmp(&right.id));
    let last_performance = performances.last_mut().ok_or("last performance")?;
    last_performance.athlete = target.id.clone();
    let expected_performance = format!("performances:{}", last_performance.id);
    let mut later_performance = last_performance.clone();
    later_performance.athlete = other.id.clone();
    let last_relay = relays.last_mut().ok_or("last relay")?;
    last_relay.members = vec![RelayMember::new(1, "Runner").with_athlete(target.id.clone())];
    let expected_relay = format!("relay_results:{}", last_relay.id);
    let mut later_relay = last_relay.clone();
    later_relay.members.clear();
    store.append_many(Table::Performances, &performances)?;
    store.append(Table::Performances, &later_performance)?;
    store.append_many(Table::RelayResults, &relays)?;
    store.append(Table::RelayResults, &later_relay)?;
    let members = vec![target.id.cast()];
    let inputs = context::Inputs::capture(&store.snapshot())?;
    let identity = store.store_identity()?;
    let root = inputs.measure_context(&identity)?;
    let affected =
        local_adjudication::affected::Inputs::capture(&store.snapshot(), &identity, &root)?;
    let expected = affected.record_keys(&inputs, &members)?;
    let outcome = writer.capture_raw_generation_v2(
        &store,
        &RawGenerationOperationIdV2::new("affected-algorithm")?,
    )?;
    let held = store.snapshot();
    let graph = VerifiedRawGraph::verify(
        &held,
        outcome.retained().binding(),
        outcome.retained().seal(),
    )?;
    let actual = record_keys(&graph, &members)?;
    check!(eq; actual, expected);
    check!(actual.contains(&expected_performance));
    check!(actual.contains(&expected_relay));
    check!(eq; actual.len(), 3);
    Ok(())
}

#[test]
fn affected_logical_key_bound_accepts_1024_and_refuses_1025() -> TestResult {
    for count in [1023, 1024] {
        let (_directory, store, writer) = enrolled()?;
        let target = subject("affected-boundary", "64001")?;
        store.append(Table::Athletes, &target)?;
        let mut performances = Vec::new();
        for index in 0..count {
            performances.push(performance(&target.id, &format!("boundary-{index}"))?);
        }
        store.append_many(Table::Performances, &performances)?;
        let outcome = writer.capture_raw_generation_v2(
            &store,
            &RawGenerationOperationIdV2::new("affected-boundary")?,
        )?;
        let held = store.snapshot();
        let graph = VerifiedRawGraph::verify(
            &held,
            outcome.retained().binding(),
            outcome.retained().seal(),
        )?;
        let actual = record_keys(&graph, &[target.id.cast()]);
        if count == 1023 {
            check!(eq; actual?.len(), 1024);
        } else {
            check!(matches!(
                actual,
                Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                    "manual affected record count bound"
                )))
            ));
        }
    }
    Ok(())
}
