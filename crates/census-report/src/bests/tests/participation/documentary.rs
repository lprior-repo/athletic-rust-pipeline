use super::*;
use crate::export::{DocumentaryParticipationRow, ParticipationDiagnostic};
use census_domain::participation_candidates::{CandidateOutcome, ParticipationReasons};
use std::collections::BTreeMap;

mod scope;

#[test]
fn documentary_outcomes_keep_actual_assessment_and_withhold_school_unresolved_rows() -> TestResult {
    let fixture = crate::export::selected_test_fixture::consumer::source_births()?;
    let (dataset, _objects) =
        crate::export::selected_test_fixture::consumer::capture(&fixture.store)?;
    let (_, observation, _) = observations()?;
    let facts = Facts::new()?;
    let reasons = facts
        .assessment(
            &observation.performance,
            &[ParticipationContext::Collegiate],
        )?
        .reasons();
    let unresolved = facts
        .assessment(
            &observation.performance,
            &[
                ParticipationContext::Scholastic,
                ParticipationContext::Collegiate,
            ],
        )?
        .reasons();
    let cases = [
        (
            CandidateOutcome::EligibleCandidate,
            ParticipationReasons::default(),
            ParticipationOutcome::Eligible,
            ParticipationExclusionOutcome::Unresolved,
            ParticipationDiagnostic::DocumentaryAssessmentEligible,
        ),
        (
            CandidateOutcome::Excluded,
            reasons,
            ParticipationOutcome::Excluded,
            ParticipationExclusionOutcome::Excluded,
            ParticipationDiagnostic::DocumentaryAssessmentExcluded,
        ),
        (
            CandidateOutcome::Unresolved,
            unresolved,
            ParticipationOutcome::Unresolved,
            ParticipationExclusionOutcome::Unresolved,
            ParticipationDiagnostic::DocumentaryAssessmentUnresolved,
        ),
    ];
    for (actual, reasons, mapped, disposition, tag) in cases {
        let id = Id::mint("perf", &["synthetic-documentary-outcome"]);
        let athlete = fixture
            .athletes
            .first()
            .ok_or("retained identity")?
            .id
            .clone();
        let row = DocumentaryParticipationRow::fixture(
            id.clone(),
            observation.event.id.clone(),
            athlete.clone(),
            actual,
            reasons,
            vec!["synthetic-source-reference".into()],
        );
        let view = view(&dataset, row)?;
        let before = dataset.performances.clone();
        let outcome = reduce(&dataset, &view, &options(&dataset)?)?;
        check!(outcome.selections.is_empty());
        check!(eq; outcome.participation_exclusions.len(), 1);
        let exclusion = outcome
            .participation_exclusions
            .first()
            .ok_or("documentary exclusion")?;
        check!(eq; exclusion.performance_id(), &id);
        check!(eq; exclusion.event_id(), &observation.event.id);
        check!(eq; exclusion.athlete_id(), &athlete);
        check!(eq; exclusion.outcome(), disposition);
        check!(eq; exclusion.reason(), ParticipationExclusionReason::DocumentarySchoolBindingUnresolved { assessment: actual, reasons });
        let retained = view
            .documentary(&id)
            .ok_or("retained documentary observation")?;
        check!(eq; retained.domain_outcome(), actual);
        check!(eq; retained.assessment().outcome(), mapped);
        check!(eq; retained.assessment().diagnostics(), &[tag, ParticipationDiagnostic::SchoolBindingUnresolved]);
        check!(eq; dataset.performances, before);
        check!(view.assessment(&id).is_none());
    }
    Ok(())
}

#[test]
fn documentary_identity_remains_excluded_when_positive_cohort_population_is_empty() -> TestResult {
    let fixture = crate::export::selected_test_fixture::consumer::source_births_with(
        crate::export::selected_test_fixture::consumer::RosterCase::Missing,
    )?;
    let (dataset, _objects) =
        crate::export::selected_test_fixture::consumer::capture(&fixture.store)?;
    let (_, observation, _) = observations()?;
    let athlete = fixture
        .athletes
        .first()
        .ok_or("retained identity")?
        .id
        .clone();
    let row = DocumentaryParticipationRow::fixture(
        observation.performance.id.clone(),
        observation.event.id.clone(),
        athlete.clone(),
        CandidateOutcome::EligibleCandidate,
        ParticipationReasons::default(),
        vec!["synthetic-identity-only-source".into()],
    );
    let view = view(&dataset, row)?;
    let options = options(&dataset)?;
    let derivation = crate::report::Derivation::of(
        &dataset,
        options.scope,
        options.grad_year,
        &options.jurisdictions,
    )?;
    check!(derivation.performances().is_empty());
    check!(derivation.athletes().is_empty());
    let outcome = reduce(&dataset, &view, &options)?;
    check!(outcome.selections.is_empty());
    check!(eq; outcome.participation_exclusions.len(), 1);
    check!(eq; outcome.participation_exclusions.first().ok_or("withheld identity")?.athlete_id(), &athlete);
    check!(dataset.performances.is_empty());
    Ok(())
}

fn view(
    dataset: &ExportDataset,
    row: DocumentaryParticipationRow,
) -> TestResult<VerifiedParticipationView> {
    let authority = dataset.selected_authority()?;
    Ok(VerifiedParticipationView::fixture(
        dataset.lineage.input_generation.clone(),
        authority.profile().id()?,
        authority.profile().input().policies.participation.clone(),
        BTreeMap::new(),
    )
    .fixture_with_documentary(
        BTreeMap::from([(row.performance_id().clone(), row)]),
        Vec::new(),
    ))
}

fn options(dataset: &ExportDataset) -> TestResult<Options> {
    let profile = dataset.selected_authority()?.profile();
    let [cohort] = profile.input().graduation_years.as_slice() else {
        return Err("one selected cohort".into());
    };
    Ok(Options {
        scope: Scope::AllSources,
        grad_year: Some(cohort.get()),
        jurisdictions: profile.input().jurisdictions.clone(),
        limit: None,
    })
}

fn reduce(
    dataset: &ExportDataset,
    view: &VerifiedParticipationView,
    options: &Options,
) -> TestResult<crate::bests::SelectionOutcome> {
    Ok(crate::bests::reduce::build_outcome(
        &crate::report::EventAdmission::of(dataset)?,
        options,
        ParticipationSelection::Selected(view),
    )?)
}
