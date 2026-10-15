use super::{
    CanonicalPerformance, ExportDataset, QueryProfileV1, Scope, SelectionOutcome, TestResult,
};
use crate::bests::{ParticipationExclusionOutcome, ParticipationExclusionReason};
use crate::export::ParticipationOutcome;
use census_domain::participation_candidates::ParticipationReason;

pub(super) fn all_retained(dataset: &ExportDataset) -> TestResult {
    check!(eq; dataset.performances.len(), 2);
    for performance in &dataset.performances {
        let assessment = dataset
            .verified_participation()?
            .assessment(&performance.id)
            .ok_or("source-backed high-school assessment")?;
        check!(eq; assessment.outcome(), ParticipationOutcome::Unresolved);
        check!(eq; assessment.reasons().iter().collect::<Vec<_>>(), vec![ParticipationReason::HighSchoolStatusMissing, ParticipationReason::SummerBoundaryUnresolved]);
        check!(assessment.diagnostics().is_empty());
    }
    let outcome = selections(dataset)?;
    check!(outcome.selections.is_empty());
    check!(eq; outcome.participation_exclusions.len(), 2);
    Ok(())
}

pub(super) fn bounded(
    dataset: &ExportDataset,
    unresolved: &CanonicalPerformance,
    excluded: &CanonicalPerformance,
    profile: &QueryProfileV1,
) -> TestResult<SelectionOutcome> {
    check!(eq; dataset.selected_profile_id(), Some(&profile.id()?));
    check!(eq; dataset.performances.len(), 2);
    check!(dataset.performances.contains(unresolved));
    check!(dataset.performances.contains(excluded));
    check!(ne; unresolved.meet, excluded.meet);
    check!(eq; unresolved.athlete, excluded.athlete);
    check!(eq; unresolved.source_key, "milesplit_result:201782263");
    check!(eq; excluded.source_key, "milesplit_result:201782264");
    check!(eq; unresolved.date, "2026-03-27");
    check!(eq; excluded.date, "2026-04-03");
    check!(eq; crate::bests::mark_text(&unresolved.mark), "13-9");
    check!(eq; crate::bests::mark_text(&excluded.mark), "14-9");
    let participation = dataset.verified_participation()?;
    let accepted = participation
        .assessment(&unresolved.id)
        .ok_or("unresolved March assessment")?;
    check!(eq; accepted.outcome(), ParticipationOutcome::Unresolved);
    check!(eq; accepted.reasons().iter().collect::<Vec<_>>(), vec![ParticipationReason::HighSchoolStatusMissing, ParticipationReason::SummerBoundaryUnresolved]);
    let rejected = participation
        .assessment(&excluded.id)
        .ok_or("outside-window April assessment")?;
    check!(eq; rejected.outcome(), ParticipationOutcome::Excluded);
    check!(eq; rejected.reasons().iter().collect::<Vec<_>>(), vec![ParticipationReason::AfterPerformanceWindow, ParticipationReason::HighSchoolStatusMissing, ParticipationReason::SummerBoundaryUnresolved]);
    for assessment in [accepted, rejected] {
        check!(assessment.diagnostics().is_empty());
        for prefix in ["acquisition/", "metadata/"] {
            check!(assessment
                .evidence_refs()
                .iter()
                .any(|value| value.starts_with(prefix)));
        }
    }
    let outcome = selections(dataset)?;
    check!(outcome.selections.is_empty());
    check!(outcome.combined_exclusions.is_empty());
    check!(eq; outcome.participation_exclusions.len(), 2);
    let rejection = outcome
        .participation_exclusions
        .iter()
        .find(|row| row.performance_id() == &excluded.id)
        .ok_or("exact window exclusion")?;
    check!(eq; rejection.performance_id(), &excluded.id);
    check!(eq; rejection.event_id(), &excluded.event);
    check!(eq; rejection.athlete_id(), &excluded.athlete);
    check!(eq; rejection.outcome(), ParticipationExclusionOutcome::Excluded);
    check!(eq; rejection.reason(), ParticipationExclusionReason::Assessment(rejected.reasons()));
    let missing = outcome
        .participation_exclusions
        .iter()
        .find(|row| row.performance_id() == &unresolved.id)
        .ok_or("missing-status/window exclusion")?;
    check!(eq; missing.outcome(), ParticipationExclusionOutcome::Unresolved);
    check!(eq; missing.reason(), ParticipationExclusionReason::Assessment(accepted.reasons()));
    Ok(outcome)
}

fn selections(dataset: &ExportDataset) -> TestResult<SelectionOutcome> {
    Ok(crate::bests::build_with_exclusions(
        dataset,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: vec![
                census_domain::UsJurisdiction::Alabama,
                census_domain::UsJurisdiction::Wisconsin,
            ],
        },
    )?)
}
