use super::{ExportDataset, Scope, TestResult};
use crate::export::{ParticipationAssessment, ParticipationOutcome, VerifiedParticipationView};
use census_domain::participation_candidates::ParticipationReasons;

pub(super) fn synthetic_eligible_candidates(dataset: &ExportDataset) -> TestResult {
    let view = VerifiedParticipationView::fixture(
        dataset.lineage.input_generation.clone(),
        dataset
            .selected_profile_id()
            .ok_or("selected profile")?
            .clone(),
        dataset
            .selected_participation_rule()
            .ok_or("selected rule")?
            .clone(),
        dataset
            .performances
            .iter()
            .map(|performance| {
                (
                    performance.id.clone(),
                    ParticipationAssessment::fixture(
                        ParticipationOutcome::Eligible,
                        ParticipationReasons::default(),
                        vec!["synthetic-reducer-unit-assumption".into()],
                    ),
                )
            })
            .collect(),
    );
    let outcome = crate::bests::build_participation_outcome(
        &super::EventAdmission::of(dataset)?,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: vec![census_domain::UsJurisdiction::Alabama],
        },
        &view,
    )?;
    let retained = dataset
        .performances
        .iter()
        .find(|row| row.source_key == "milesplit_result:201782263")
        .ok_or("eligible-scope performance")?;
    check!(eq; outcome.selections.len(), 1);
    let best = outcome
        .selections
        .first()
        .ok_or("selected-scope fallback")?;
    check!(eq; best.source.performance_id, retained.id);
    check!(eq; best.population.marks, 1);
    check!(eq; best.inventory.eligible_group_count(), 1);
    check!(eq; best.inventory.tied_group_count(), 1);
    check!(eq; best.inventory.tied_representative_ids(), std::slice::from_ref(&retained.id));
    check!(outcome.participation_exclusions.is_empty());
    check!(outcome.combined_exclusions.is_empty());
    Ok(())
}
