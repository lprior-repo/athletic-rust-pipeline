use super::*;
use crate::public_identity::{authority, context, local_adjudication};
use census_domain::model::{SourceAthleteObservation, SourceNamespace};

#[test]
fn reviewed_decision_and_material_match_historical_fixture() -> TestResult {
    let fixture = context::reviewed_fixture::ready_for_merge()?;
    fixture
        .writer
        .enroll_graph_admission_v1(&fixture.store, fixture.store.snapshot().revision())?;
    let inputs = context::Inputs::capture(&fixture.store.snapshot())?;
    let case_id = fixture.request.case_id();
    let expected = authority::accept_reviewed(&inputs, case_id, "2026-10-15")?;
    let expected_material = authority::identity_material(&inputs, &fixture.candidates)?;
    let outcome = fixture.writer.capture_raw_generation_v2(
        &fixture.store,
        &RawGenerationOperationIdV2::new("reviewed-algorithm")?,
    )?;
    let held = fixture.store.snapshot();
    let graph = VerifiedRawGraph::verify(
        &held,
        outcome.retained().binding(),
        outcome.retained().seal(),
    )?;
    check!(eq; accept_reviewed(&graph, case_id, "2026-10-15")?.record(), expected.record());
    check!(eq; identity_material(&graph, &fixture.candidates)?, expected_material);
    check!(eq; review_case(&graph, case_id)?.id, case_id);
    let ledger = fixture.writer.verified(&fixture.store)?;
    let public = fixture.births.first().ok_or("birth")?.public_id();
    check!(eq; members(&graph, ledger.history.projection(), public)?,
        local_adjudication::prepare::members(&inputs, ledger.history.projection(), public)?);
    Ok(())
}

#[test]
fn claims_preserve_full_merged_and_physical_ordinals_and_row_digests() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    let mut athletes = Vec::new();
    for index in 0..4 {
        athletes.push(subject(&format!("claim-{index}"), &format!("61{index}"))?);
    }
    athletes.sort_by(|left, right| left.id.cmp(&right.id));
    let target = athletes.last().ok_or("target")?.clone();
    store.append_many(Table::Athletes, &athletes)?;
    store.append(Table::Athletes, &target)?;
    let source = target.source.as_ref().ok_or("source")?;
    let observation = SourceObservation::Athlete(SourceAthleteObservation::new(
        SourceNamespace::MilesplitAthlete,
        &source.id,
        "claim-physical",
        "Runner",
        "2026-10-15",
    ));
    let unrelated = SourceObservation::Athlete(SourceAthleteObservation::new(
        SourceNamespace::MilesplitAthlete,
        "99999",
        "unrelated-physical",
        "Other",
        "2026-10-15",
    ));
    store.append_many(
        Table::SourceObservations,
        &[unrelated, observation.clone(), observation],
    )?;
    let members = vec![target.id.cast()];
    let inputs = context::Inputs::capture(&store.snapshot())?;
    let ledger = writer.verified(&store)?;
    let expected = local_adjudication::claims::membership_inventory(
        &inputs,
        &members,
        ledger.history.projection(),
        ledger.history.records(),
    )?;
    let outcome = writer
        .capture_raw_generation_v2(&store, &RawGenerationOperationIdV2::new("claim-algorithm")?)?;
    let held = store.snapshot();
    let graph = VerifiedRawGraph::verify(
        &held,
        outcome.retained().binding(),
        outcome.retained().seal(),
    )?;
    let actual = membership_inventory(
        &graph,
        &members,
        ledger.history.projection(),
        ledger.history.records(),
    )?;
    check!(eq; actual, expected);
    check!(eq; member_rows(&graph, &members)?.first().ok_or("member row")?.0, 3);
    let observations = actual.iter().filter(|claim| claim.section ==
        census_domain::public_identity_history::local_adjudication::RetainedIdentitySectionV1::SourceObservation).count();
    check!(eq; observations, 2);
    Ok(())
}

#[test]
fn repeated_primary_without_epoch_explanation_still_refuses() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    let first = subject("epoch-first", "62001")?;
    let second = subject("epoch-second", "62001")?;
    let members = vec![first.id.cast(), second.id.cast()];
    store.append_many(Table::Athletes, &[first, second])?;
    let inputs = context::Inputs::capture(&store.snapshot())?;
    let ledger = writer.verified(&store)?;
    let expected = local_adjudication::claims::membership_inventory(
        &inputs,
        &members,
        ledger.history.projection(),
        ledger.history.records(),
    );
    let outcome = writer
        .capture_raw_generation_v2(&store, &RawGenerationOperationIdV2::new("epoch-algorithm")?)?;
    let held = store.snapshot();
    let graph = VerifiedRawGraph::verify(
        &held,
        outcome.retained().binding(),
        outcome.retained().seal(),
    )?;
    let actual = membership_inventory(
        &graph,
        &members,
        ledger.history.projection(),
        ledger.history.records(),
    );
    check!(matches!(
        expected,
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "manual correction needs source epoch evidence"
        )))
    ));
    check!(matches!(
        actual,
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "manual correction needs source epoch evidence"
        )))
    ));
    Ok(())
}
