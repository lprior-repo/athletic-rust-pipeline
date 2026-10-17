use super::{close, fold, support, Folded, PrKey, PrSlot, RetainedContexts};
use crate::bests::exclusions::Exclusions;
use crate::bests::participation::{self, ParticipationSelection};
use crate::bests::{Options, Parents, SelectionOutcome, SharedSelection};
use crate::report::{Derivation, EventAdmission, ReportResult};
use std::collections::HashMap;

pub fn build_from_dataset(
    dataset: &crate::export::ExportDataset,
    options: &Options,
) -> ReportResult<Vec<SharedSelection>> {
    Ok(build_with_exclusions(dataset, options)?.selections)
}

pub fn build_with_exclusions(
    dataset: &crate::export::ExportDataset,
    options: &Options,
) -> ReportResult<SelectionOutcome> {
    let admission = EventAdmission::of(dataset)?;
    build_admitted_outcome(&admission, options)
}

pub(crate) fn build_admitted_outcome(
    admission: &EventAdmission<'_>,
    options: &Options,
) -> ReportResult<SelectionOutcome> {
    let derivation = Derivation::for_publication(
        admission.dataset(),
        options.scope,
        options.grad_year,
        &options.jurisdictions,
    )?;
    reduce_derivation(
        admission,
        &derivation,
        ParticipationSelection::of(admission.dataset())?,
        options.limit,
    )
}

#[cfg(test)]
pub(in crate::bests) fn build_outcome(
    admission: &EventAdmission<'_>,
    options: &Options,
    participation: ParticipationSelection<'_>,
) -> ReportResult<SelectionOutcome> {
    Derivation::with_participation_reducer_fixture(admission.dataset(), options, |derivation| {
        reduce_derivation(admission, derivation, participation, options.limit)
    })
}

pub(crate) fn build_from_derivation(
    derivation: &Derivation<'_>,
    admission: &EventAdmission<'_>,
    limit: Option<usize>,
) -> ReportResult<SelectionOutcome> {
    derivation.ensure_selected_affiliation()?;
    reduce_derivation(
        admission,
        derivation,
        ParticipationSelection::of(derivation.dataset())?,
        limit,
    )
}

pub(crate) fn build_selected_outcome(
    derivation: &Derivation<'_>,
    admission: &EventAdmission<'_>,
    participation: &crate::export::VerifiedParticipationView,
    limit: Option<usize>,
) -> ReportResult<SelectionOutcome> {
    derivation.ensure_selected_affiliation()?;
    reduce_derivation(
        admission,
        derivation,
        ParticipationSelection::Selected(participation),
        limit,
    )
}

fn reduce_derivation(
    admission: &EventAdmission<'_>,
    derivation: &Derivation<'_>,
    participation: ParticipationSelection<'_>,
    limit: Option<usize>,
) -> ReportResult<SelectionOutcome> {
    admission.ensure_dataset(derivation.dataset())?;
    participation.ensure_dataset(derivation.dataset())?;
    super::identity::admit(&admission.dataset().performances, admission.semantics())?;
    support::admit(derivation.performances())?;
    let parents = Parents::of(derivation);
    let retained = RetainedContexts::of(derivation.performances(), &parents, admission);
    let mut slots: HashMap<PrKey, PrSlot<'_>> = HashMap::new();
    let mut exclusions = Exclusions::new();
    let mut participation_exclusions = participation::Exclusions::new();
    derivation
        .performances()
        .iter()
        .copied()
        .try_for_each(|performance| -> ReportResult<()> {
            if participation_exclusions.admit(participation, performance)?
                == participation::CandidateAdmission::Withheld
            {
                return Ok(());
            }
            match fold(&mut slots, performance, &parents, &retained, admission) {
                Ok(Folded::Ignored) => Ok(()),
                Ok(Folded::Eligible) => exclusions.admit(performance),
                Err(reason) => exclusions.record(performance, reason),
            }
        })?;
    participation_exclusions.admit_documentary(participation, derivation, limit)?;
    let selections = crate::bests::selection::publish(
        slots
            .into_iter()
            .filter_map(|(key, slot)| {
                close(
                    key,
                    slot,
                    &parents,
                    derivation.scope(),
                    admission.semantics(),
                )
            })
            .collect(),
        limit,
    );
    Ok(SelectionOutcome {
        selections,
        combined_exclusions: exclusions.finish()?,
        participation_exclusions: participation_exclusions.finish(),
    })
}
