use super::{close, fold, support, Folded, PrKey, PrSlot, RetainedContexts};
use crate::bests::exclusions::Exclusions;
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
    super::identity::admit(&admission.dataset().performances, admission.semantics())?;
    let derivation = Derivation::of(
        admission.dataset(),
        options.scope,
        options.grad_year,
        &options.jurisdictions,
    );
    support::admit(derivation.performances())?;
    let parents = Parents::of(&derivation);
    let retained = RetainedContexts::of(derivation.performances(), &parents, admission);
    let mut slots: HashMap<PrKey, PrSlot<'_>> = HashMap::new();
    let mut exclusions = Exclusions::new();
    derivation
        .performances()
        .iter()
        .copied()
        .try_for_each(|performance| -> ReportResult<()> {
            match fold(&mut slots, performance, &parents, &retained, admission) {
                Ok(Folded::Ignored) => Ok(()),
                Ok(Folded::Eligible) => exclusions.admit(performance),
                Err(reason) => exclusions.record(performance, reason),
            }
        })?;
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
        options.limit,
    );
    Ok(SelectionOutcome {
        selections,
        combined_exclusions: exclusions.finish()?,
    })
}
