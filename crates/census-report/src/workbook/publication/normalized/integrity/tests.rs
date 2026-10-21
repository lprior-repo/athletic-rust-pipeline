use super::*;
use crate::export::public_identity::tests::fixture::{merge, source_births, TestResult};
use crate::export::{ParticipationAssessment, ParticipationOutcome, VerifiedParticipationView};
use crate::report::Scope;
use census_domain::model::{IdentityStatus, RetainedConflict, ATHLETE_IDENTITY_FAMILY};
use census_domain::participation_candidates::ParticipationReasons;
use census_store::Table;
use std::collections::BTreeMap;

use crate::workbook::selected_identity_fixture as fixture;
mod result;

#[test]
fn committed_public_merge_admits_best_and_ties_when_automatic_projection_keeps_candidates_separate(
) -> TestResult {
    let source = source_births()?;
    merge(&source)?;
    let [first, second, _] = &source.athletes;
    let mut retained = source
        .store
        .snapshot()
        .athletes()?
        .into_iter()
        .find(|athlete| athlete.id == first.id)
        .ok_or("first retained candidate")?;
    retained.retained_conflicts.push(RetainedConflict::new(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        "Synthetic retained source conflict",
        "Automatic source projection stays separate from the committed public merge",
    ));
    source.store.append(Table::Athletes, &retained)?;
    let mut dataset = fixture::selected_dataset(&source.store)?;
    result::append(&source.store, &mut dataset, first, "first", "2026-05-01")?;
    result::append(&source.store, &mut dataset, second, "second", "2026-05-02")?;
    let projection = dataset.identities();
    check!(eq; projection.status(first.id.as_str())?, IdentityStatus::RetainedConflict);
    check!(ne; projection.canonical_id(first.id.as_str()), projection.canonical_id(second.id.as_str()));
    let derivation = Derivation::for_publication(&dataset, Scope::AllSources, None, &[])?;
    let admission = EventAdmission::of(&dataset)?;
    let participation = eligible(&dataset)?;
    let outcome =
        crate::bests::build_selected_outcome(&derivation, &admission, &participation, None)?;
    let [best] = outcome.selections.as_slice() else {
        return Err("public merge did not produce one selected best".into());
    };
    check!(eq; best.athlete_id(), &first.id);
    check!(eq; best.supporting_tied_candidate_ids().len(), 2);
    let performances = validate(&derivation, &admission, &outcome.selections)?;
    check!(eq; performances.len(), 2);
    let parents = Parents::of(&derivation);
    ties::validate(best, &derivation, &admission, &performances, &parents)?;
    let mut swapped = best.clone();
    swapped.key.athlete_id = second.id.clone();
    check!(validate(&derivation, &admission, &[swapped]).is_err());
    Ok(())
}

#[test]
fn selected_tie_readback_rejects_a_matching_mark_from_an_unknown_identity() -> TestResult {
    let source = source_births()?;
    merge(&source)?;
    let [first, second, unknown] = &source.athletes;
    let mut dataset = fixture::selected_dataset(&source.store)?;
    result::append(&source.store, &mut dataset, first, "first", "2026-05-01")?;
    result::append(&source.store, &mut dataset, second, "second", "2026-05-02")?;
    let derivation = Derivation::for_publication(&dataset, Scope::AllSources, None, &[])?;
    let admission = EventAdmission::of(&dataset)?;
    let participation = eligible(&dataset)?;
    let outcome =
        crate::bests::build_selected_outcome(&derivation, &admission, &participation, None)?;
    let [best] = outcome.selections.as_slice() else {
        return Err("selected tied best".into());
    };
    let mut corrupted = dataset.performances.clone();
    let alias = corrupted
        .iter_mut()
        .find(|row| row.athlete == second.id)
        .ok_or("second tied performance")?;
    alias.athlete = unknown.id.clone();
    let performances: PerformanceIndex<'_> = corrupted.iter().map(|row| (&row.id, row)).collect();
    let parents = Parents::of(&derivation);
    check!(ties::validate(best, &derivation, &admission, &performances, &parents).is_err());
    Ok(())
}

fn eligible(dataset: &crate::export::ExportDataset) -> TestResult<VerifiedParticipationView> {
    Ok(VerifiedParticipationView::fixture(
        dataset.lineage.input_generation.clone(),
        dataset
            .selected_profile_id()
            .ok_or("selected profile")?
            .clone(),
        dataset
            .selected_participation_rule()
            .ok_or("participation rule")?
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
                        vec![format!("synthetic-eligible-{}", performance.id)],
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>(),
    ))
}
