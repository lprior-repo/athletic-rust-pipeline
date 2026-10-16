use super::*;
use census_domain::model::{ContractRefV1, ProfileId};

#[test]
fn selected_participation_requires_an_explicit_profile_binding() -> TestResult {
    let (fixture, slower, _) = observations()?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[slower.clone()])?;
    let dataset = ExportDataset::load(&store)?;
    let facts = Facts::new()?;
    let view = facts.view(&dataset, [eligible(&facts, &slower.performance)?]);
    refused(
        &dataset,
        &view,
        "participation belongs to a different or missing selected profile",
    )
}

#[test]
fn selected_participation_requires_an_explicit_rule_binding() -> TestResult {
    let (fixture, slower, _) = observations()?;
    let facts = Facts::new()?;
    let directory = tempfile::tempdir()?;
    let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
    let (dataset, _objects) = facts.bind(&store, &writer, &[slower.clone()])?;
    let foreign = ContractRefV1::new(1, 2, &"a".repeat(64))?;
    check!(dataset.selected_participation_rule() != Some(&foreign));
    let view = VerifiedParticipationView::fixture(
        dataset.lineage.input_generation.clone(),
        dataset
            .selected_profile_id()
            .ok_or("selected profile")?
            .clone(),
        foreign,
        [eligible(&facts, &slower.performance)?]
            .into_iter()
            .collect(),
    );
    refused(
        &dataset,
        &view,
        "participation belongs to a different or missing selected rule",
    )
}

#[test]
fn selected_participation_rejects_another_profile_on_the_same_frozen_input() -> TestResult {
    let (fixture, slower, _) = observations()?;
    let facts = Facts::new()?;
    let directory = tempfile::tempdir()?;
    let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
    let (dataset, _objects) = facts.bind(&store, &writer, &[slower.clone()])?;
    let profile = ProfileId::parse(&"b".repeat(64))?;
    check!(dataset.selected_profile_id() != Some(&profile));
    let view = VerifiedParticipationView::fixture(
        dataset.lineage.input_generation.clone(),
        profile,
        dataset
            .selected_participation_rule()
            .ok_or("selected rule")?
            .clone(),
        [eligible(&facts, &slower.performance)?]
            .into_iter()
            .collect(),
    );
    refused(
        &dataset,
        &view,
        "participation belongs to a different or missing selected profile",
    )
}

#[test]
fn selected_participation_checks_the_exact_rule_on_the_same_frozen_input() -> TestResult {
    let (fixture, slower, _) = observations()?;
    let facts = Facts::new()?;
    let directory = tempfile::tempdir()?;
    let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
    let (dataset, _objects) = facts.bind(&store, &writer, &[slower.clone()])?;
    for rule in [
        ContractRefV1::new(1, 2, &"a".repeat(64))?,
        ContractRefV1::new(1, 1, &"b".repeat(64))?,
    ] {
        check!(dataset.selected_participation_rule() != Some(&rule));
        let view = VerifiedParticipationView::fixture(
            dataset.lineage.input_generation.clone(),
            dataset
                .selected_profile_id()
                .ok_or("selected profile")?
                .clone(),
            rule,
            [eligible(&facts, &slower.performance)?]
                .into_iter()
                .collect(),
        );
        refused(
            &dataset,
            &view,
            "participation belongs to a different or missing selected rule",
        )?;
    }
    Ok(())
}

#[test]
fn selected_bound_inputs_cannot_use_historical_participation_admission() -> TestResult {
    let (fixture, slower, _) = observations()?;
    let facts = Facts::new()?;
    let directory = tempfile::tempdir()?;
    let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
    let (mut dataset, _objects) = facts.bind(&store, &writer, &[slower.clone()])?;
    for revision in [18, 19, 20] {
        dataset.lineage.policy_revision = revision;
        check!(matches!(
            ParticipationSelection::of(&dataset),
            Err(crate::report::ReportError::Invariant { detail })
                if detail == "selected participation requires current comparison semantics"
        ));
        check!(matches!(
            ParticipationSelection::Historical.ensure_dataset(&dataset),
            Err(crate::report::ReportError::Invariant { detail })
                if detail == "selected participation policy cannot use historical admission"
        ));
    }
    dataset.lineage.policy_revision = 21;
    check!(ParticipationSelection::of(&dataset).is_ok());
    check!(matches!(
        ParticipationSelection::Historical.ensure_dataset(&dataset),
        Err(crate::report::ReportError::Invariant { detail })
            if detail == "selected participation policy cannot use historical admission"
    ));
    Ok(())
}

fn eligible(
    facts: &Facts,
    performance: &CanonicalPerformance,
) -> TestResult<(PerformanceId, ParticipationAssessment)> {
    Ok((
        performance.id.clone(),
        facts.assessment(performance, &[ParticipationContext::Scholastic])?,
    ))
}

fn refused(
    dataset: &ExportDataset,
    view: &VerifiedParticipationView,
    expected: &str,
) -> TestResult {
    match selected(dataset, view, None) {
        Err(error) => check!(error.to_string().contains(expected)),
        Ok(_) => return Err("foreign or unbound participation reached numeric selection".into()),
    }
    Ok(())
}
