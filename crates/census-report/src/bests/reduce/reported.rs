use super::{fold, identity, support, Folded, PrKey, PrSlot, ReportGroup, RetainedContexts};
use crate::bests::exclusions::Exclusions;
use crate::bests::Parents;
use crate::export::ExportDataset;
use crate::report::{EventAdmission, ReportError, ReportResult};
use census_domain::model::{CanonicalPerformance, PerformanceId};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReportedBestGroup {
    pub(crate) key: PrKey,
    pub(crate) candidate_id: Option<PerformanceId>,
    pub(crate) conflicts: Vec<crate::bests::Conflict>,
}

pub(crate) fn reported_best_groups(
    dataset: &ExportDataset,
    admission: &EventAdmission<'_>,
    performances: &[&CanonicalPerformance],
) -> ReportResult<Vec<ReportedBestGroup>> {
    admission.ensure_dataset(dataset)?;
    identity::admit(&dataset.performances, admission.semantics())?;
    support::admit(performances)?;
    let range = dataset.performances.as_ptr_range();
    if performances
        .iter()
        .any(|row| !range.contains(&std::ptr::from_ref(*row)))
    {
        return Err(ReportError::Invariant {
            detail: "reported performance is not a row of the admitted dataset".into(),
        });
    }
    let parents = Parents::for_reported(dataset);
    let retained = RetainedContexts::of(performances, &parents, admission);
    let mut slots: HashMap<PrKey, PrSlot<'_>> = HashMap::new();
    let mut exclusions = Exclusions::new();
    for performance in performances.iter().copied() {
        match fold(&mut slots, performance, &parents, &retained, admission) {
            Ok(Folded::Ignored) => {}
            Ok(Folded::Eligible) => exclusions.admit(performance)?,
            Err(reason) => exclusions.record(performance, reason)?,
        }
    }
    let _ = exclusions.finish()?;
    let mut groups: Vec<_> = slots
        .into_iter()
        .map(|(key, slot)| {
            let candidate_id = slot
                .candidate()
                .map(|candidate| candidate.performance.id.clone());
            ReportedBestGroup {
                key,
                candidate_id,
                conflicts: slot
                    .reports
                    .into_values()
                    .filter_map(ReportGroup::conflict)
                    .collect(),
            }
        })
        .collect();
    groups.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(groups)
}
