use super::*;
use crate::bests::SelectionOutcome;
use census_domain::model::RetainedConflict;

#[test]
fn selected_time_distance_and_points_exclude_better_and_tied_ineligible_groups() -> TestResult {
    for case in numeric_cases()? {
        for mark in [case.tied.clone(), case.better.clone()] {
            let fixture = TieFixture::new()?;
            let school = fixture.report(&case, "school", "2026-05-02")?;
            let collegiate = fixture
                .report(&case, "collegiate", "2026-05-01")?
                .with_mark(mark)?;
            let facts = Facts::new()?;
            let directory = tempfile::tempdir()?;
            let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
            let (dataset, _objects) =
                facts.bind(&store, &writer, &[school.clone(), collegiate.clone()])?;
            let view = facts.view(
                &dataset,
                [
                    assessed(&facts, &school, ParticipationContext::Scholastic)?,
                    assessed(&facts, &collegiate, ParticipationContext::Collegiate)?,
                ],
            );
            let before = dataset.performances.clone();
            let outcome = selected(&dataset, &view, None)?;
            check!(eq; dataset.performances, before);
            check!(eq; outcome.selections.len(), 1);
            let best = outcome.selections.first().ok_or("school winner")?;
            check!(eq; best.source.performance_id, school.performance.id);
            check!(eq; best.result.value, case.value);
            check!(eq; best.population.marks, 1);
            check!(eq; best.population.sources, 1);
            check!(eq; best.inventory.eligible_group_count(), 1);
            check!(eq; best.inventory.tied_group_count(), 1);
            check!(eq; best.inventory.tied_representative_ids(), &[school.performance.id.clone()]);
            check!(eq; best.supporting_tied_candidate_ids(), &[school.performance.id.clone()]);
            check!(outcome.combined_exclusions.is_empty());
            check!(eq; outcome.participation_exclusions.len(), 1);
            let excluded = outcome
                .participation_exclusions
                .first()
                .ok_or("collegiate exclusion")?;
            check!(eq; excluded.performance_id(), &collegiate.performance.id);
            check!(eq; excluded.outcome(), ParticipationExclusionOutcome::Excluded);
            check!(eq; reasons(excluded), vec![ParticipationReason::CollegiateParticipation]);
        }
    }
    Ok(())
}

#[test]
fn accepted_participation_keeps_retained_conflicts_and_uses_the_clean_fallback() -> TestResult {
    let (fixture, slower, faster) = observations()?;
    let mut conflicted = faster.clone();
    conflicted
        .performance
        .retained_conflicts
        .push(RetainedConflict::new(
            "performance",
            faster.performance.id.as_str(),
            "retained result",
            "source correction remains contested",
        ));
    let facts = Facts::new()?;
    let directory = tempfile::tempdir()?;
    let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
    let (dataset, _objects) = facts.bind(
        &store,
        &writer,
        &[slower.clone(), faster.clone(), conflicted.clone()],
    )?;
    let mut expected = None;
    for _ in 0..2 {
        let view = facts.view(
            &dataset,
            [
                assessed(&facts, &slower, ParticipationContext::Scholastic)?,
                assessed(&facts, &faster, ParticipationContext::Scholastic)?,
            ],
        );
        let before = dataset.performances.clone();
        let outcome = selected(&dataset, &view, None)?;
        check!(eq; dataset.performances, before);
        check!(eq; outcome.selections.len(), 1);
        let best = outcome.selections.first().ok_or("clean fallback")?;
        check!(eq; best.source.performance_id, slower.performance.id);
        check!(eq; best.inventory.eligible_group_count(), 1);
        check!(eq; best.inventory.tied_group_count(), 1);
        check!(eq; best.supporting_tied_candidate_ids(), &[slower.performance.id.clone()]);
        check!(eq; best.conflicts.len(), 1);
        check!(eq; best.conflicts.first().ok_or("retained conflict")?.marks,
            vec![crate::bests::mark_text(&faster.performance.mark)]);
        check!(outcome.participation_exclusions.is_empty());
        check!(outcome.combined_exclusions.is_empty());
        same_outcome(&mut expected, outcome)?;
    }
    Ok(())
}

#[test]
fn mixed_participation_duplicates_and_permutations_keep_exact_tie_inventory() -> TestResult {
    let fixture = TieFixture::new()?;
    let case = numeric_cases()?.into_iter().next().ok_or("time case")?;
    let first = fixture.report(&case, "first-school", "2026-05-01")?;
    let second = fixture.report(&case, "second-school", "2026-05-02")?;
    let excluded = fixture.report(&case, "excluded-tie", "2026-04-30")?;
    let missing = fixture
        .report(&case, "missing-proof", "2026-04-29")?
        .with_mark(case.better.clone())?;
    let observations = [
        first.clone(),
        second.clone(),
        excluded.clone(),
        missing.clone(),
    ];
    let facts = Facts::new()?;
    let mut tied = vec![first.performance.id.clone(), second.performance.id.clone()];
    tied.sort();
    let mut expected = None;
    for order in super::super::ties::fixture::orders(&observations)? {
        let directory = tempfile::tempdir()?;
        let order_observations: Vec<Observation> = order.into_iter().map(|row| row).collect();
        let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
        let (dataset, _objects) = facts.bind(&store, &writer, &order_observations)?;
        let view = facts.view(
            &dataset,
            [
                assessed(&facts, &first, ParticipationContext::Scholastic)?,
                assessed(&facts, &second, ParticipationContext::Scholastic)?,
                assessed(&facts, &excluded, ParticipationContext::Collegiate)?,
            ],
        );
        let outcome = selected(&dataset, &view, None)?;
        check!(eq; outcome.selections.len(), 1);
        let best = outcome.selections.first().ok_or("tied winner")?;
        check!(eq; best.source.performance_id, first.performance.id);
        check!(eq; best.population.marks, 2);
        check!(eq; best.population.sources, 1);
        check!(eq; best.inventory.eligible_group_count(), 2);
        check!(eq; best.inventory.tied_group_count(), 2);
        check!(eq; best.inventory.tied_representative_ids(), tied.as_slice());
        check!(eq; best.supporting_tied_candidate_ids(), tied.as_slice());
        check!(eq; outcome.participation_exclusions.len(), 2);
        let missing = outcome
            .participation_exclusions
            .iter()
            .find(|row| row.performance_id() == &missing.performance.id)
            .ok_or("missing-proof exclusion")?;
        check!(eq; missing.outcome(), ParticipationExclusionOutcome::Unresolved);
        check!(eq; missing.reason(), ParticipationExclusionReason::MissingParticipationEvidence);
        let excluded = outcome
            .participation_exclusions
            .iter()
            .find(|row| row.performance_id() == &excluded.performance.id)
            .ok_or("collegiate exclusion")?;
        check!(eq; excluded.outcome(), ParticipationExclusionOutcome::Excluded);
        check!(eq; reasons(excluded), vec![ParticipationReason::CollegiateParticipation]);
        same_outcome(&mut expected, outcome)?;
    }
    Ok(())
}

fn assessed(
    facts: &Facts,
    observation: &Observation,
    context: ParticipationContext,
) -> TestResult<(PerformanceId, ParticipationAssessment)> {
    Ok((
        observation.performance.id.clone(),
        facts.assessment(&observation.performance, &[context])?,
    ))
}

fn same_outcome(expected: &mut Option<SelectionOutcome>, actual: SelectionOutcome) -> TestResult {
    match expected {
        Some(expected) => check!(eq; &actual, &*expected),
        None => *expected = Some(actual),
    }
    Ok(())
}
