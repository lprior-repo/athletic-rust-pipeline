use crate::export::qualification::physical_observations::OBSERVATION_BUDGET as PROOF_BUDGET;
use crate::report::ReportResult;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam,
};
use census_store::{Entity, ReadBudget, StoreSnapshot, Table};

pub(super) fn admit(snapshot: &StoreSnapshot<'_>) -> ReportResult<()> {
    admit_with_budget(snapshot, PROOF_BUDGET)
}

fn admit_with_budget(snapshot: &StoreSnapshot<'_>, budget: ReadBudget) -> ReportResult<()> {
    table::<CanonicalAthlete>(snapshot, Table::Athletes, budget)?;
    table::<CanonicalPerformance>(snapshot, Table::Performances, budget)?;
    table::<CanonicalTeam>(snapshot, Table::Teams, budget)?;
    table::<CanonicalMeet>(snapshot, Table::Meets, budget)?;
    table::<CanonicalEvent>(snapshot, Table::Events, budget)
}

fn table<T: Entity>(
    snapshot: &StoreSnapshot<'_>,
    table: Table,
    budget: ReadBudget,
) -> ReportResult<()> {
    snapshot.for_each_observation_bounded::<T>(table, budget, |_, _| Ok(()))?;
    Ok(())
}

#[cfg(test)]
#[path = "preflight/tests.rs"]
mod tests;
