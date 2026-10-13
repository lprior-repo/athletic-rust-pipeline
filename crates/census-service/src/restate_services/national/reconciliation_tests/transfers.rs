use super::{drive, fixtures, TestResult};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::sync::Arc;

#[tokio::test]
async fn merge_and_school_transfer_replay_and_restore_source_owners() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Arc::new(Store::open(directory.path())?);
    let rows = fixtures::seed_transfer(&store)?;
    let first = rows.first().ok_or("missing original owner")?;
    let transferred = rows.last().ok_or("missing transferred owner")?;
    check!(first.school != transferred.school);
    let performance = historical_performance(&store, first)?;
    let journal = drive(Arc::clone(&store)).await?;
    let projection = store.athlete_identity_projection()?;
    let canonical = rows
        .iter()
        .map(|row| row.id.as_str())
        .min()
        .ok_or("missing root")?;
    for row in &rows {
        check!(eq; projection.canonical_id(row.id.as_str()), canonical);
        check!(eq; projection.status(row.id.as_str())?, IdentityStatus::Verified);
    }
    check!(eq; projection.alias_transitions().len(), 2);
    let decisions = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    let decision = decisions
        .iter()
        .find(|record| record.kind == AppliedIdentityKind::SamePerson)
        .ok_or("missing durable merge and transfer decision")?;
    check!(eq; decision.members.len(), 3);
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    let verdicts = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    let transitions = projection.alias_transitions().to_vec();
    drop(store);
    let store = Store::open(directory.path())?;
    let replay = store.athlete_identity_projection()?;
    check!(eq; replay.alias_transitions(), transitions.as_slice());
    let mut expected_prior: Vec<_> = rows
        .iter()
        .map(|row| row.id.as_str())
        .filter(|id| *id != canonical)
        .collect();
    expected_prior.sort();
    let prior_owners: Vec<_> = replay
        .alias_transitions()
        .iter()
        .map(IdentityAliasTransition::prior_canonical_id)
        .collect();
    check!(eq; prior_owners, expected_prior);
    for transition in replay.alias_transitions() {
        check!(eq; transition.decision_id(), decision.id.as_str());
        check!(eq; transition.canonical_id(), canonical);
        println!(
            "61k invocation={} decision={} subject={} prior_canonical={} canonical={}",
            journal.invocation,
            transition.decision_id(),
            transition.subject(),
            transition.prior_canonical_id(),
            transition.canonical_id()
        );
    }
    let reversed = AthleteIdentityProjection::replay_without(
        store.athlete_identity_index()?,
        &cases,
        &verdicts,
        &decisions,
        &decision.id,
    )?;
    for row in &rows {
        check!(eq; reversed.canonical_id(row.id.as_str()), row.id.as_str());
        println!(
            "61k restored_subject={} source_school={}",
            row.id, row.school
        );
    }
    check!(eq; reversed.alias_transitions().len(), 0);
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?, decisions);
    let retained = store.scan::<CanonicalAthlete>(Table::Athletes)?;
    check!(eq; retained.len(), rows.len());
    for row in &rows {
        check!(retained.contains(row));
    }
    check!(eq; store.scan::<CanonicalPerformance>(Table::Performances)?, vec![performance]);
    Ok(())
}

fn historical_performance(
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
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(
            &athlete.school,
            Sport::OutdoorTrack,
            Gender::Girls,
            SchoolYear::new(2025).ok_or("missing fixture year")?,
        ),
        school: athlete.school.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Girls,
        school_year: SchoolYear::new(2025).ok_or("missing fixture year")?,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![evidence.clone()],
        retained_conflicts: Vec::new(),
    };
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
    let performance = CanonicalPerformance {
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
    };
    store.append(Table::Meets, &meet)?;
    store.append(Table::Teams, &team)?;
    store.append(Table::Events, &event)?;
    store.append(Table::Performances, &performance)?;
    Ok(performance)
}
