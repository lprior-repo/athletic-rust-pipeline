use super::{
    CanonicalPerformance, ExportDataset, QueryProfileV1, Scope, SelectionOutcome, TestResult,
};
use crate::bests::{ParticipationExclusionOutcome, ParticipationExclusionReason};
use crate::export::ParticipationOutcome;
use census_domain::model::performance_comparison::{
    ComparisonPolicy, Measure, PrKey, SurfaceClass, TimingClass, WindClass,
};
use census_domain::model::{CompetitionCategory, EventKind, EventSpecification};
use census_domain::participation_candidates::ParticipationReason;

pub(super) fn all_retained(dataset: &ExportDataset, stronger: &CanonicalPerformance) -> TestResult {
    check!(eq; dataset.performances.len(), 2);
    for performance in &dataset.performances {
        let assessment = dataset
            .verified_participation()?
            .assessment(&performance.id)
            .ok_or("independently supported in-season assessment")?;
        check!(eq; assessment.outcome(), ParticipationOutcome::Eligible, "all-retained participation: {}", performance.source_key);
        check!(assessment.reasons().iter().next().is_none());
        check!(assessment.diagnostics().is_empty());
    }
    let outcome = selections(dataset)?;
    check!(outcome.participation_exclusions.is_empty());
    check!(outcome.combined_exclusions.is_empty());
    selected(&outcome, stronger, 4_495_800, 2)
}

pub(super) fn bounded(
    dataset: &ExportDataset,
    eligible: &CanonicalPerformance,
    excluded: &CanonicalPerformance,
    profile: &QueryProfileV1,
) -> TestResult<SelectionOutcome> {
    check!(eq; dataset.selected_profile_id(), Some(&profile.id()?));
    check!(eq; dataset.performances.len(), 2);
    check!(dataset.performances.contains(eligible));
    check!(dataset.performances.contains(excluded));
    check!(ne; eligible.meet, excluded.meet);
    check!(ne; eligible.event, excluded.event, "March-window distinct physical events");
    check!(eq; eligible.athlete, excluded.athlete);
    check!(eq; eligible.source_key, "milesplit_result:201782263");
    check!(eq; excluded.source_key, "milesplit_result:201782264");
    check!(eq; eligible.date, "2026-03-27");
    check!(eq; excluded.date, "2026-04-03");
    check!(eq; crate::bests::mark_text(&eligible.mark), "13-9");
    check!(eq; crate::bests::mark_text(&excluded.mark), "14-9");
    let participation = dataset.verified_participation()?;
    let accepted = participation
        .assessment(&eligible.id)
        .ok_or("eligible March assessment")?;
    check!(eq; accepted.outcome(), ParticipationOutcome::Eligible);
    check!(accepted.reasons().iter().next().is_none());
    let rejected = participation
        .assessment(&excluded.id)
        .ok_or("outside-window April assessment")?;
    check!(eq; rejected.outcome(), ParticipationOutcome::Excluded);
    check!(eq; rejected.reasons().iter().collect::<Vec<_>>(), vec![ParticipationReason::AfterPerformanceWindow]);
    check!(accepted.diagnostics().is_empty());
    check!(rejected.diagnostics().is_empty());
    let outcome = selections(dataset)?;
    selected(&outcome, eligible, 4_191_000, 1)?;
    check!(outcome.combined_exclusions.is_empty());
    check!(eq; outcome.participation_exclusions.len(), 1);
    let rejection = outcome
        .participation_exclusions
        .first()
        .ok_or("exact window exclusion")?;
    check!(eq; rejection.performance_id(), &excluded.id);
    check!(eq; rejection.event_id(), &excluded.event);
    check!(eq; rejection.athlete_id(), &excluded.athlete);
    check!(eq; rejection.outcome(), ParticipationExclusionOutcome::Excluded);
    check!(eq; rejection.reason(), ParticipationExclusionReason::Assessment(rejected.reasons()));
    Ok(outcome)
}

fn selected(
    outcome: &SelectionOutcome,
    expected: &CanonicalPerformance,
    value: i64,
    eligible_groups: usize,
) -> TestResult {
    check!(eq; outcome.selections.len(), 1, "PR selection count for {} with {eligible_groups} eligible groups", expected.source_key);
    let row = outcome
        .selections
        .first()
        .ok_or("nonempty actual PR winner")?;
    check!(eq; row.key, PrKey {
        athlete_id: expected.athlete.clone(),
        event_kind: EventKind::LongJump,
        surface: SurfaceClass::Outdoor,
        wind_class: WindClass::Unknown,
        timing: TimingClass::NonTime,
        measure: Measure::Distance,
        context: None,
        specification: EventSpecification {
            category: Some(CompetitionCategory::Published("boys/high school".into())),
            ..EventSpecification::default()
        },
        comparison: ComparisonPolicy::Standard,
        adjustment: None,
    }, "exact common PR comparison class for {}", expected.source_key);
    check!(eq; row.source.performance_id, expected.id);
    check!(eq; row.source.source_key, expected.source_key);
    check!(eq; row.athlete_id(), &expected.athlete);
    check!(eq; row.result.value, value);
    check!(eq; row.result.mark, expected.mark);
    check!(eq; row.meet.date, expected.date);
    check!(eq; row.meet.meet_id, expected.meet);
    check!(eq; row.inventory.eligible_group_count(), eligible_groups, "PR eligible result-group count for {}", expected.source_key);
    check!(eq; row.inventory.tied_group_count(), 1);
    check!(eq; row.inventory.tied_representative_ids(), std::slice::from_ref(&expected.id));
    check!(eq; row.inventory.policy().revision(), 22);
    Ok(())
}

fn selections(dataset: &ExportDataset) -> TestResult<SelectionOutcome> {
    Ok(crate::bests::build_with_exclusions(
        dataset,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: vec![census_domain::UsJurisdiction::Michigan],
        },
    )?)
}
