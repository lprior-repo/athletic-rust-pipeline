use super::*;
use crate::bests::{build_with_exclusions, CombinedExclusionReason, SelectionOutcome};
use census_domain::model::{CompetitionCategory, SpecificationError};

mod scope;

pub(super) fn selection_outcome(dataset: &ExportDataset) -> TestResult<SelectionOutcome> {
    Ok(build_with_exclusions(
        dataset,
        &Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: Vec::new(),
        },
    )?)
}

pub(super) fn assert_exclusion(
    outcome: &SelectionOutcome,
    performance: &CanonicalPerformance,
    reason: CombinedEligibility,
) -> TestResult {
    check!(eq; outcome.combined_exclusions.len(), 1);
    let exclusion = outcome
        .combined_exclusions
        .first()
        .ok_or("missing combined exclusion")?;
    check!(eq; exclusion.performance_id(), &performance.id);
    check!(eq; exclusion.event_id(), &performance.event);
    check!(eq; exclusion.athlete_id(), &performance.athlete);
    check!(eq; exclusion.reason(), CombinedExclusionReason::Eligibility(reason));
    Ok(())
}

fn missing_completion(fixture: &TieFixture, label: &str) -> TestResult<Observation> {
    let mut observation =
        pentathlon_report(fixture, label, 500000, Some(pentathlon_specification(1)?))?;
    observation.performance.combined_completion = None;
    Ok(observation)
}

#[test]
fn combined_exclusions_survive_zero_winners_without_mutating_source_evidence() -> TestResult {
    let fixture = TieFixture::new()?;
    let observation = missing_completion(&fixture, "zero-winners")?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&observation))?;
    let dataset = ExportDataset::load(&store)?;
    let before = dataset.performances.clone();
    let outcome = selection_outcome(&dataset)?;
    check!(outcome.selections.is_empty());
    assert_exclusion(
        &outcome,
        &observation.performance,
        CombinedEligibility::MissingCompletion,
    )?;
    check!(eq; selected(&dataset)?, outcome.selections);
    check!(eq; dataset.performances, before);
    Ok(())
}

#[test]
fn combined_exclusions_keep_historical_nineteen_semantics() -> TestResult {
    let fixture = TieFixture::new()?;
    let observation = missing_completion(&fixture, "historical-completion")?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&observation))?;
    let mut dataset = ExportDataset::load(&store)?;
    {
        dataset.lineage.policy_revision = 19;
        let outcome = selection_outcome(&dataset)?;
        check!(outcome.combined_exclusions.is_empty());
        check!(eq; outcome.selections.len(), 1);
        check!(eq; outcome.selections.first().ok_or("missing historical winner")?.source.performance_id,
            observation.performance.id);
        check!(eq; selected(&dataset)?, outcome.selections);
    }
    Ok(())
}

#[test]
fn combined_exclusions_distinguish_unresolved_specification_from_missing_completion() -> TestResult
{
    let fixture = TieFixture::new()?;
    let observation = missing_completion(&fixture, "unresolved-specification")?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&observation))?;
    let mut dataset = ExportDataset::load(&store)?;
    let event = dataset.events.first_mut().ok_or("missing combined event")?;
    event.specification.category = Some(CompetitionCategory::Girls);
    let outcome = selection_outcome(&dataset)?;
    check!(outcome.selections.is_empty());
    check!(eq; outcome.combined_exclusions.len(), 1);
    let exclusion = outcome
        .combined_exclusions
        .first()
        .ok_or("missing specification exclusion")?;
    check!(eq; exclusion.performance_id(), &observation.performance.id);
    check!(eq; exclusion.reason(), CombinedExclusionReason::UnresolvedSpecification(
        SpecificationError::ConflictingSpecification));
    Ok(())
}

#[test]
fn combined_exclusions_are_stable_and_deduplicated_across_replay() -> TestResult {
    let fixture = TieFixture::new()?;
    let first = missing_completion(&fixture, "excluded-first")?;
    let second = pentathlon_report(&fixture, "excluded-second", 500000, None)?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[first.clone(), second.clone()])?;
    let mut dataset = ExportDataset::load(&store)?;
    let expected = selection_outcome(&dataset)?;
    check!(eq; expected.combined_exclusions.len(), 2);
    check!(expected.selections.is_empty());
    dataset.performances.push(first.performance);
    check!(eq; selection_outcome(&dataset)?, expected);
    dataset.performances.reverse();
    check!(eq; selection_outcome(&dataset)?, expected);
    Ok(())
}

#[test]
fn combined_exclusions_respect_the_existing_cohort_scope() -> TestResult {
    let fixture = TieFixture::new()?;
    let observation = missing_completion(&fixture, "out-of-cohort")?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&observation))?;
    let dataset = ExportDataset::load(&store)?;
    let outcome = build_with_exclusions(
        &dataset,
        &Options {
            scope: Scope::AllSources,
            grad_year: Some(2028),
            limit: None,
            jurisdictions: Vec::new(),
        },
    )?;
    check!(outcome.selections.is_empty());
    check!(outcome.combined_exclusions.is_empty());
    Ok(())
}

#[test]
fn combined_exclusions_refuse_one_id_with_eligible_and_excluded_facts() -> TestResult {
    let fixture = TieFixture::new()?;
    let eligible = pentathlon_report(
        &fixture,
        "mixed-facts",
        500000,
        Some(pentathlon_specification(1)?),
    )?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&eligible))?;
    let mut dataset = ExportDataset::load(&store)?;
    let mut excluded = eligible.performance.clone();
    excluded.combined_completion = None;
    dataset.performances.push(excluded);
    for _ in 0..2 {
        let result = build_with_exclusions(&dataset, &Options::default());
        check!(
            matches!(result, Err(crate::report::ReportError::Invariant { detail })
            if detail.contains("both eligible and excluded"))
        );
        dataset.performances.reverse();
    }
    Ok(())
}

#[test]
fn combined_exclusions_preserve_replay_when_duplicate_id_is_eligible() -> TestResult {
    let fixture = TieFixture::new()?;
    let eligible = pentathlon_report(
        &fixture,
        "eligible-replay",
        500000,
        Some(pentathlon_specification(1)?),
    )?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&eligible))?;
    let mut dataset = ExportDataset::load(&store)?;
    let expected = selection_outcome(&dataset)?;
    dataset.performances.push(eligible.performance);
    let replay = selection_outcome(&dataset)?;
    check!(replay.combined_exclusions.is_empty());
    check!(eq; replay.selections.len(), 1);
    let expected = expected
        .selections
        .first()
        .ok_or("missing expected winner")?;
    let actual = replay.selections.first().ok_or("missing replay winner")?;
    check!(eq; actual.source.performance_id, expected.source.performance_id);
    check!(eq; actual.supporting_tied_candidate_ids(), expected.supporting_tied_candidate_ids());
    check!(eq; actual.inventory.tied_representative_ids(), expected.inventory.tied_representative_ids());
    Ok(())
}

#[test]
fn combined_exclusions_refuse_one_id_with_conflicting_excluded_facts_in_both_orders() -> TestResult
{
    let fixture = TieFixture::new()?;
    let missing = missing_completion(&fixture, "conflicting-excluded-facts")?;
    let mut mismatched = missing.performance.clone();
    mismatched.combined_completion = Some(completion_with_total(
        pentathlon_specification(1)?,
        CentiPoints::new(499999),
    )?);
    check!(eq; missing.performance.combined_eligibility(&missing.event.specification),
        CombinedEligibility::MissingCompletion);
    check!(eq; mismatched.combined_eligibility(&missing.event.specification),
        CombinedEligibility::TotalMismatch);
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&missing))?;
    let mut dataset = ExportDataset::load(&store)?;
    dataset.performances.push(mismatched);
    let expected = format!(
        "combined exclusion {} has conflicting candidate facts",
        missing.performance.id
    );
    for _ in 0..2 {
        match build_with_exclusions(&dataset, &Options::default()) {
            Err(crate::report::ReportError::Invariant { detail }) => {
                check!(eq; detail, expected);
            }
            Err(error) => return Err(format!("unexpected refusal: {error}").into()),
            Ok(_) => return Err("conflicting excluded facts were silently deduplicated".into()),
        }
        dataset.performances.reverse();
    }
    Ok(())
}
