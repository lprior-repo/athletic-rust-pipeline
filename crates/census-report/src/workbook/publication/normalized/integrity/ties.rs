use super::{defect, matches_key, unique, PerformanceIndex};
use crate::bests::{Parents, SharedSelection};
use crate::export::public_identity::population::IdentityPopulation;
use crate::report::{Derivation, EventAdmission, ReportResult};

pub(super) fn validate(
    best: &SharedSelection,
    derivation: &Derivation<'_>,
    admission: &EventAdmission<'_>,
    performances: &PerformanceIndex<'_>,
    parents: &Parents<'_>,
) -> ReportResult<()> {
    if best.inventory.policy() != admission.semantics() {
        return Err(defect(
            "shared best inventory has a different selection policy",
        ));
    }
    inventory(best)?;
    let identities = IdentityPopulation::of(derivation.dataset())?;
    best.supporting_tied_candidate_ids()
        .iter()
        .try_for_each(|id| {
            let performance = performances
                .get(id)
                .copied()
                .ok_or_else(|| defect(format!("tie references unavailable candidate {id}")))?;
            if identities.canonical_candidate(performance.athlete.as_str())
                != Some(best.athlete_id().as_str())
                || best.measure().value(&performance.mark) != Some(best.result.value)
                || !matches_key(performance, best, admission, parents)?
            {
                return Err(defect(format!(
                    "tie {id} differs from selected athlete, class or mark"
                )));
            }
            Ok(())
        })
}

fn inventory(best: &SharedSelection) -> ReportResult<()> {
    if best.inventory.eligible_group_count() == 0
        || best.inventory.tied_group_count() == 0
        || best.inventory.tied_group_count() > best.inventory.eligible_group_count()
        || best.inventory.tied_group_count() != best.inventory.tied_representative_ids().len()
        || !best
            .inventory
            .tied_representative_ids()
            .contains(&best.source.performance_id)
    {
        return Err(defect("invalid shared best candidate/tie inventory"));
    }
    unique(
        best.inventory
            .tied_representative_ids()
            .iter()
            .map(|id| id.as_str()),
        "tied performance",
    )?;
    let supporting = best.supporting_tied_candidate_ids();
    let ordered = supporting.windows(2).all(|pair| match pair {
        [left, right] => left < right,
        _ => false,
    });
    if !ordered
        || !best
            .inventory
            .tied_representative_ids()
            .iter()
            .all(|id| supporting.binary_search(id).is_ok())
    {
        return Err(defect("invalid supporting tied candidate inventory"));
    }
    Ok(())
}
