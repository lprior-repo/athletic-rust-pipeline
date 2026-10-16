use super::super::{bound, invalid, MAX_BUNDLE, MAX_PACKET};
use crate::milesplit::ProviderSchools;
use census_domain::model::CanonicalSchool;
use census_store::{ReadBudget, StoreError, StoreResult, StoreSnapshot, Table};

pub(super) fn read(
    snapshot: &StoreSnapshot<'_>,
    budget: ReadBudget,
) -> StoreResult<ProviderSchools> {
    let rows = usize::try_from(budget.rows).map_err(|_| StoreError::CounterOverflow)?;
    let bytes = usize::try_from(budget.bytes).map_err(|_| StoreError::CounterOverflow)?;
    bound(rows, crate::SCHOOL_BINDINGS, "provider target school rows")?;
    bound(bytes, MAX_BUNDLE, "provider target school scan bytes")?;
    bound(
        budget.record_bytes,
        MAX_PACKET,
        "provider target school record bytes",
    )?;
    if rows == 0 || bytes == 0 || budget.record_bytes == 0 {
        return Err(invalid(
            "provider target school scan requires a nonzero budget",
        ));
    }
    let mut schools = ProviderSchools::default();
    let mut admitted = 0usize;
    snapshot.for_each_merged_bounded(Table::Schools, budget, |school: CanonicalSchool, _| {
        admitted = admitted
            .checked_add(school.source_identities.len())
            .ok_or(StoreError::CounterOverflow)?;
        bound(
            admitted,
            crate::SCHOOL_BINDINGS,
            "provider target school index entries",
        )?;
        schools.absorb(&school);
        bound(
            schools.entries(),
            admitted,
            "provider target admitted school entries",
        )
    })?;
    Ok(schools)
}
