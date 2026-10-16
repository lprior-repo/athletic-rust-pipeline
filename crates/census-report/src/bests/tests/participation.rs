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
        let facts = Facts::new()?;
        let directory = tempfile::tempdir()?;
        let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
        let (dataset, _objects) = facts.bind(&store, &writer, &[slower.clone(), faster.clone()])?;
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
    let facts = Facts::new()?;
    let directory = tempfile::tempdir()?;
    let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
    let (dataset, _objects) = facts.bind(&store, &writer, &[slower.clone(), faster.clone()])?;
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
    let (fixture, slower, faster) = observations()?;
    let facts = Facts::new()?;
    let directory = tempfile::tempdir()?;
    let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
    let (dataset, _objects) = facts.bind(&store, &writer, &[slower.clone(), faster.clone()])?;
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
    let mut missing = faster.clone();
    missing.performance.date.clear();
    let missing_directory = tempfile::tempdir()?;
    let (missing_store, missing_writer) =
        fixture.store_with_selected_replay(missing_directory.path())?;
    let (missing_dataset, _missing_objects) = facts.bind(
        &missing_store,
        &missing_writer,
        &[slower.clone(), missing.clone()],
    )?;
    let missing_date =
        facts.assessment(&missing.performance, &[ParticipationContext::Scholastic])?;
    let view = facts.view(
        &missing_dataset,
        [(missing.performance.id.clone(), missing_date)],
    );
    let outcome = selected(&missing_dataset, &view, None)?;
    let missing = outcome
        .participation_exclusions
        .iter()
        .find(|row| row.performance_id() == &missing.performance.id)
        .ok_or("missing date")?;
    check!(eq; missing.outcome(), ParticipationExclusionOutcome::Unresolved);
    check!(eq; reasons(missing), vec![ParticipationReason::MissingPerformanceDate]);
    Ok(())
}

#[test]
fn selected_bests_do_not_accept_missing_assessments_or_hide_them_at_zero_row_limit() -> TestResult {
    let (fixture, slower, faster) = observations()?;
    let facts = Facts::new()?;
    let directory = tempfile::tempdir()?;
    let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
    let (dataset, _objects) = facts.bind(&store, &writer, &[slower.clone(), faster.clone()])?;
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
    let reverse_directory = tempfile::tempdir()?;
    let (reverse_store, reverse_writer) =
        fixture.store_with_selected_replay(reverse_directory.path())?;
    let (reverse_dataset, _reverse_objects) = facts.bind(
        &reverse_store,
        &reverse_writer,
        &[faster.clone(), slower.clone()],
    )?;
    let reverse_view = facts.view(&reverse_dataset, []);
    check!(eq; selected(&reverse_dataset, &reverse_view, None)?, expected);
    Ok(())
}

#[test]
fn selected_bests_refuse_a_verified_view_from_a_different_frozen_input() -> TestResult {
    let (fixture, slower, _) = observations()?;
    let facts = Facts::new()?;
    let directory = tempfile::tempdir()?;
    let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
    let (dataset, _objects) = facts.bind(&store, &writer, &[slower.clone()])?;
    let view = facts.view(&dataset, []);
    let foreign = VerifiedParticipationView::fixture(
        "different-frozen-input".into(),
        view.profile_id().clone(),
        view.participation_rule().clone(),
        std::collections::BTreeMap::new(),
    );
    match selected(&dataset, &foreign, None) {
        Err(error) => check!(error.to_string().contains("different frozen input")),
        Ok(_) => return Err("foreign participation evidence was accepted".into()),
    }
    check!(
        selected(&dataset, &view, None)?
            .participation_exclusions
            .len()
            == 1
    );
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
    let mut terminal = slower.clone();
    terminal.performance.mark = Mark::Raw("DQ".into());
    let facts = Facts::new()?;
    let directory = tempfile::tempdir()?;
    let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
    let (dataset, _objects) = facts.bind(&store, &writer, &[slower.clone(), terminal.clone()])?;
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
    let facts = Facts::new()?;
    let directory = tempfile::tempdir()?;
    let (store, writer) = fixture.store_with_selected_replay(directory.path())?;
    let (dataset, _objects) = facts.bind(&store, &writer, &[slower.clone(), faster.clone()])?;
    let path = directory.path().join("frozen-input.json");
    let digest = dataset.archive_digest()?;
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
    let mut newer = faster.clone();
    newer.performance.id = Id::mint("perf", &["newer-participation-evidence"]);
    newer.performance.source_key = "newer-participation-evidence".into();
    let reopened =
        match ExportDataset::reopen_frozen_with_digest(&path, &digest) {
            Err(error) => {
                check!(error
                    .to_string()
                    .contains("frozen affiliation descriptor differs from its bound input"));
                None
            }
            Ok(reopened) => {
                check!(eq; reopened.performances, dataset.performances);
                check!(eq; reopened.lineage.input_generation, dataset.lineage.input_generation);
                match selected(&reopened, &view, None) {
                    Err(error) => check!(error.to_string().contains(
                        "selected publication requires retained participation verification",
                    )),
                    Ok(_) => return Err(
                        "synthetic participation views survived a frozen roundtrip without proofs"
                            .into(),
                    ),
                }
                Some(reopened)
            }
        };
    check!(eq; std::fs::read(&path)?, original_bytes);
    check!(eq; selected(&dataset, &view, None)?, expected);
    let baseline = reopened
        .as_ref()
        .map_or(&dataset.lineage.input_generation, |reopened| {
            &reopened.lineage.input_generation
        })
        .clone();
    let (fresh, _fresh_objects) =
        facts.bind(&store, &writer, &[slower.clone(), faster.clone(), newer])?;
    check!(fresh.lineage.input_generation != baseline);
    match selected(&fresh, &view, None) {
        Err(error) => check!(error.to_string().contains("different frozen input")),
        Ok(_) => return Err("old view admitted newly acquired evidence".into()),
    }
    Ok(())
}
