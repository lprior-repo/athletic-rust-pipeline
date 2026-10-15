use super::*;
use census_domain::participation_candidates::ParticipationReasons;

#[test]
fn evidenced_scholastic_club_open_and_unattached_results_survive_on_the_summer_boundary(
) -> TestResult {
    [
        ParticipationContext::Scholastic,
        ParticipationContext::Club,
        ParticipationContext::Open,
        ParticipationContext::Unattached,
    ]
    .into_iter()
    .try_for_each(|context| -> TestResult {
        let (fixture, _, mut faster) = observations()?;
        faster.performance.date = "2026-07-31".into();
        let directory = tempfile::tempdir()?;
        let store = fixture.store_with_replay(directory.path(), &[faster.clone()])?;
        let mut dataset = ExportDataset::load(&store)?;
        let facts = Facts::new()?;
        facts.bind(&mut dataset)?;
        let view = facts.view(
            &dataset,
            [(
                faster.performance.id.clone(),
                facts.assessment(&faster.performance, &[context])?,
            )],
        );
        let outcome = selected(&dataset, &view, None)?;
        check!(eq; outcome.selections.len(), 1);
        let winner = outcome.selections.first().ok_or("boundary winner")?;
        check!(eq; winner.source.performance_id, faster.performance.id);
        check!(eq; winner.inventory.eligible_group_count(), 1);
        check!(eq; winner.inventory.tied_group_count(), 1);
        check!(outcome.participation_exclusions.is_empty());
        Ok(())
    })
}

#[test]
fn a_result_after_the_profile_cutoff_retains_every_exclusion_reason() -> TestResult {
    let (fixture, _, mut faster) = observations()?;
    faster.performance.date = "2026-08-16".into();
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[faster.clone()])?;
    let mut dataset = ExportDataset::load(&store)?;
    let facts = Facts::new()?;
    facts.bind(&mut dataset)?;
    let view = facts.view(
        &dataset,
        [(
            faster.performance.id.clone(),
            facts.assessment(&faster.performance, &[ParticipationContext::Scholastic])?,
        )],
    );
    let outcome = selected(&dataset, &view, None)?;
    check!(outcome.selections.is_empty());
    check!(eq; reasons(outcome.participation_exclusions.first().ok_or("cutoff exclusion")?),
        vec![ParticipationReason::AfterEffectiveCutoff, ParticipationReason::OutsideParticipationWindow]);
    Ok(())
}

#[test]
fn inconsistent_verified_outcomes_cannot_be_used_to_bypass_selection() -> TestResult {
    let (fixture, _, faster) = observations()?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[faster.clone()])?;
    let mut dataset = ExportDataset::load(&store)?;
    let facts = Facts::new()?;
    facts.bind(&mut dataset)?;
    let excluded = facts.assessment(&faster.performance, &[ParticipationContext::Collegiate])?;
    [
        (ParticipationOutcome::Eligible, excluded.reasons()),
        (
            ParticipationOutcome::Excluded,
            ParticipationReasons::default(),
        ),
        (
            ParticipationOutcome::Unresolved,
            ParticipationReasons::default(),
        ),
    ]
    .into_iter()
    .try_for_each(|(outcome, reasons)| -> TestResult {
        let view = facts.view(
            &dataset,
            [(
                faster.performance.id.clone(),
                ParticipationAssessment::fixture(outcome, reasons, vec!["source-proof".into()]),
            )],
        );
        match selected(&dataset, &view, None) {
            Err(error) => check!(error
                .to_string()
                .contains("outcome contradicts its retained reasons")),
            Ok(_) => return Err("inconsistent acceptance reached numeric fold".into()),
        }
        Ok(())
    })
}

#[test]
fn shared_derivation_selection_reuses_the_same_frozen_event_admission() -> TestResult {
    let (fixture, slower, _) = observations()?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[slower])?;
    let dataset = ExportDataset::load(&store)?;
    let foreign = ExportDataset::load(&store)?;
    let derivation = crate::report::Derivation::of(&dataset, Scope::AllSources, Some(2027), &[])?;
    let admission = crate::report::EventAdmission::of(&dataset)?;
    let outcome = crate::bests::build_from_derivation(&derivation, &admission, None)?;
    check!(eq; outcome, crate::bests::build_with_exclusions(&dataset, &Options { scope: Scope::AllSources, ..Options::default() })?);
    let foreign_admission = crate::report::EventAdmission::of(&foreign)?;
    match crate::bests::build_from_derivation(&derivation, &foreign_admission, None) {
        Err(crate::report::ReportError::Invariant { detail }) => {
            check!(eq; detail, "event admission belongs to another dataset")
        }
        Err(error) => {
            return Err(format!("unexpected foreign event admission error: {error}").into())
        }
        Ok(_) => return Err("foreign event admission was accepted".into()),
    }
    Ok(())
}
