use super::ties::fixture::{numeric_cases, Observation, TieFixture};
use super::*;
use crate::bests::participation::ParticipationSelection;
use crate::bests::{ParticipationExclusionOutcome, ParticipationExclusionReason};
use crate::export::{ParticipationAssessment, ParticipationOutcome, VerifiedParticipationView};
use census_domain::participation_candidates::{ParticipationContext, ParticipationReason};

mod binding;
mod boundaries;
mod fixture;
mod regression;
use fixture::{observations, reasons, selected, Facts};

#[test]
fn selected_bests_exclude_faster_collegiate_middle_school_and_post_graduation_results() -> TestResult
{
    [
        (
            ParticipationContext::Collegiate,
            ParticipationReason::CollegiateParticipation,
        ),
        (
            ParticipationContext::MiddleSchool,
            ParticipationReason::MiddleSchoolParticipation,
        ),
        (
            ParticipationContext::PostGraduation,
            ParticipationReason::PostGraduationParticipation,
        ),
    ]
    .into_iter()
    .try_for_each(|(context, reason)| -> TestResult {
        let (fixture, slower, faster) = observations()?;
        let directory = tempfile::tempdir()?;
        let store =
            fixture.store_with_replay(directory.path(), &[slower.clone(), faster.clone()])?;
        let mut dataset = ExportDataset::load(&store)?;
        let facts = Facts::new()?;
        facts.bind(&mut dataset)?;
        let view = facts.view(
            &dataset,
            [
                (
                    slower.performance.id.clone(),
                    facts.assessment(&slower.performance, &[ParticipationContext::Scholastic])?,
                ),
                (
                    faster.performance.id.clone(),
                    facts.assessment(&faster.performance, &[context])?,
                ),
            ],
        );
        let before = dataset.performances.clone();
        let outcome = selected(&dataset, &view, None)?;
        check!(eq; outcome.selections.len(), 1);
        let winner = outcome.selections.first().ok_or("eligible winner")?;
        check!(eq; winner.source.performance_id, slower.performance.id);
        check!(eq; winner.population.marks, 1);
        check!(eq; winner.inventory.eligible_group_count(), 1);
        check!(eq; outcome.participation_exclusions.len(), 1);
        let excluded = outcome
            .participation_exclusions
            .first()
            .ok_or("excluded result")?;
        check!(eq; excluded.performance_id(), &faster.performance.id);
        check!(eq; excluded.event_id(), &faster.performance.event);
        check!(eq; excluded.athlete_id(), &faster.performance.athlete);
        check!(eq; excluded.outcome(), ParticipationExclusionOutcome::Excluded);
        check!(eq; reasons(excluded), vec![reason]);
        check!(eq; dataset.performances, before);
        Ok(())
    })
}

#[test]
fn selected_bests_apply_the_evidenced_summer_endpoint_without_inventing_a_graduation_cutoff(
) -> TestResult {
    let (fixture, slower, mut faster) = observations()?;
    faster.performance.date = "2026-08-01".into();
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[slower.clone(), faster.clone()])?;
    let mut dataset = ExportDataset::load(&store)?;
    let facts = Facts::new()?;
    facts.bind(&mut dataset)?;
    let view = facts.view(
        &dataset,
        [
            (
                slower.performance.id.clone(),
                facts.assessment(&slower.performance, &[ParticipationContext::Scholastic])?,
            ),
            (
                faster.performance.id.clone(),
                facts.assessment(&faster.performance, &[ParticipationContext::Club])?,
            ),
        ],
    );
    let outcome = selected(&dataset, &view, None)?;
    check!(eq; outcome.selections.first().ok_or("school winner")?.source.performance_id, slower.performance.id);
    check!(eq; reasons(outcome.participation_exclusions.first().ok_or("window exclusion")?),
        vec![ParticipationReason::OutsideParticipationWindow]);
    Ok(())
}

#[test]
fn selected_bests_keep_missing_date_and_conflicting_context_reasons() -> TestResult {
    let (fixture, slower, mut faster) = observations()?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[slower.clone(), faster.clone()])?;
    let mut dataset = ExportDataset::load(&store)?;
    let facts = Facts::new()?;
    facts.bind(&mut dataset)?;
    let conflict = facts.assessment(
        &faster.performance,
        &[
            ParticipationContext::Scholastic,
            ParticipationContext::Collegiate,
        ],
    )?;
    let view = facts.view(&dataset, [(faster.performance.id.clone(), conflict)]);
    let outcome = selected(&dataset, &view, None)?;
    check!(outcome.selections.is_empty());
    let conflict = outcome
        .participation_exclusions
        .iter()
        .find(|row| row.performance_id() == &faster.performance.id)
        .ok_or("conflict")?;
    check!(eq; conflict.outcome(), ParticipationExclusionOutcome::Unresolved);
    check!(eq; reasons(conflict), vec![ParticipationReason::ParticipationEvidenceConflict, ParticipationReason::CollegiateParticipation]);
    faster.performance.date.clear();
    dataset
        .performances
        .iter_mut()
        .filter(|row| row.id == faster.performance.id)
        .for_each(|row| row.date.clear());
    facts.bind(&mut dataset)?;
    let missing_date =
        facts.assessment(&faster.performance, &[ParticipationContext::Scholastic])?;
    let view = facts.view(&dataset, [(faster.performance.id.clone(), missing_date)]);
    let outcome = selected(&dataset, &view, None)?;
    let missing = outcome
        .participation_exclusions
        .iter()
        .find(|row| row.performance_id() == &faster.performance.id)
        .ok_or("missing date")?;
    check!(eq; missing.outcome(), ParticipationExclusionOutcome::Unresolved);
    check!(eq; reasons(missing), vec![ParticipationReason::MissingPerformanceDate]);
    Ok(())
}

#[test]
fn selected_bests_do_not_accept_missing_assessments_or_hide_them_at_zero_row_limit() -> TestResult {
    let (fixture, slower, faster) = observations()?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[slower.clone(), faster.clone()])?;
    let mut dataset = ExportDataset::load(&store)?;
    let facts = Facts::new()?;
    facts.bind(&mut dataset)?;
    let view = facts.view(&dataset, []);
    let expected = selected(&dataset, &view, None)?;
    check!(expected.selections.is_empty());
    check!(eq; expected.participation_exclusions.len(), 2);
    expected
        .participation_exclusions
        .iter()
        .try_for_each(|row| -> TestResult {
            check!(eq; row.outcome(), ParticipationExclusionOutcome::Unresolved);
            check!(eq; row.reason(), ParticipationExclusionReason::MissingParticipationEvidence);
            Ok(())
        })?;
    check!(eq; selected(&dataset, &view, Some(0))?, expected);
    dataset.performances.push(faster.performance);
    dataset.performances.reverse();
    facts.bind(&mut dataset)?;
    let view = facts.view(&dataset, []);
    check!(eq; selected(&dataset, &view, None)?, expected);
    Ok(())
}

#[test]
fn selected_bests_refuse_a_verified_view_from_a_different_frozen_input() -> TestResult {
    let (fixture, slower, _) = observations()?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[slower])?;
    let mut dataset = ExportDataset::load(&store)?;
    let facts = Facts::new()?;
    facts.bind(&mut dataset)?;
    let view = facts.view(&dataset, []);
    dataset.lineage.input_generation = "different-frozen-input".into();
    match selected(&dataset, &view, None) {
        Err(error) => check!(error.to_string().contains("different frozen input")),
        Ok(_) => return Err("foreign participation evidence was accepted".into()),
    }
    Ok(())
}

#[test]
fn historical_bests_keep_old_policy_semantics_without_implied_participation_acceptance(
) -> TestResult {
    let (fixture, slower, faster) = observations()?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[slower, faster.clone()])?;
    let mut dataset = ExportDataset::load(&store)?;
    for policy in [18, 19, 20, 21] {
        dataset.lineage.policy_revision = policy;
        let outcome = crate::bests::build_with_exclusions(
            &dataset,
            &Options {
                scope: Scope::AllSources,
                ..Options::default()
            },
        )?;
        check!(eq; outcome.selections.first().ok_or("historical best")?.source.performance_id, faster.performance.id);
        check!(outcome.participation_exclusions.is_empty());
    }
    dataset.lineage.policy_revision = 22;
    match ParticipationSelection::of(&dataset) {
        Err(crate::report::ReportError::Invariant { detail }) => {
            check!(eq; detail, "selected participation policy requires verified frozen evidence")
        }
        Err(error) => return Err(format!("unexpected policy refusal: {error}").into()),
        Ok(_) => return Err("selected input silently became historical".into()),
    }
    Ok(())
}

#[test]
fn accepted_participation_does_not_clear_a_terminal_same_result_conflict() -> TestResult {
    let (fixture, slower, _) = observations()?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[slower.clone()])?;
    let mut dataset = ExportDataset::load(&store)?;
    let mut terminal = slower.performance.clone();
    terminal.mark = Mark::Raw("DQ".into());
    dataset.performances.push(terminal.clone());
    let facts = Facts::new()?;
    facts.bind(&mut dataset)?;
    let view = facts.view(
        &dataset,
        [(
            slower.performance.id.clone(),
            facts.assessment(&slower.performance, &[ParticipationContext::Scholastic])?,
        )],
    );
    let outcome = selected(&dataset, &view, None)?;
    check!(outcome.selections.is_empty());
    check!(outcome.participation_exclusions.is_empty());
    Ok(())
}

#[test]
fn selected_bests_replay_the_same_frozen_input_after_newer_store_observations() -> TestResult {
    let (fixture, slower, faster) = observations()?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[slower.clone(), faster.clone()])?;
    let mut dataset = ExportDataset::load(&store)?;
    let facts = Facts::new()?;
    facts.bind(&mut dataset)?;
    let path = directory.path().join("frozen-input.json");
    dataset.save_frozen(&path)?;
    let original_bytes = std::fs::read(&path)?;
    let view = facts.view(
        &dataset,
        [
            (
                slower.performance.id.clone(),
                facts.assessment(&slower.performance, &[ParticipationContext::Scholastic])?,
            ),
            (
                faster.performance.id.clone(),
                facts.assessment(&faster.performance, &[ParticipationContext::Collegiate])?,
            ),
        ],
    );
    let expected = selected(&dataset, &view, None)?;
    let mut newer = faster.performance.clone();
    newer.id = Id::mint("perf", &["newer-participation-evidence"]);
    newer.source_key = "newer-participation-evidence".into();
    store.append(Table::Performances, &newer)?;
    let reopened = ExportDataset::reopen_frozen(&path)?;
    check!(eq; selected(&reopened, &view, None)?, expected);
    check!(eq; std::fs::read(&path)?, original_bytes);
    check!(eq; reopened.performances, dataset.performances);
    let fresh = ExportDataset::load(&store)?;
    check!(fresh.lineage.input_generation != reopened.lineage.input_generation);
    match selected(&fresh, &view, None) {
        Err(error) => check!(error.to_string().contains("different frozen input")),
        Ok(_) => return Err("old view admitted newly acquired evidence".into()),
    }
    Ok(())
}
