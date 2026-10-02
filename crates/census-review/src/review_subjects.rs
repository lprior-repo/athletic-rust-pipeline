use std::collections::{HashMap, HashSet};

use census_domain::model::{CanonicalAthlete, ReviewCase};
use census_store::{Entity, StoreResult, StoreSnapshot, Table};

use crate::athlete_flags::key;
use crate::review_budget::Budget;

pub(super) fn selected<T: Entity>(
    snapshot: &StoreSnapshot<'_>,
    table: Table,
    ids: &HashSet<&str>,
    budget: &mut Budget,
) -> StoreResult<HashMap<String, T>> {
    let mut rows = HashMap::new();
    if ids.is_empty() {
        return Ok(rows);
    }
    snapshot.for_each_merged(table, |row: T| {
        if ids.contains(row.entity_id()) {
            budget.charge(&row)?;
            rows.insert(row.entity_id().to_string(), row);
        }
        Ok(())
    })?;
    Ok(rows)
}

pub(super) fn current_cases(
    snapshot: &StoreSnapshot<'_>,
    pending: &[(ReviewCase, crate::ReviewFamily)],
    budget: &mut Budget,
) -> StoreResult<HashMap<String, ReviewCase>> {
    let ids = pending.iter().map(|(case, _)| case.id.as_str()).collect();
    selected(snapshot, Table::ReviewCases, &ids, budget)
}

pub(super) fn athletes(
    snapshot: &StoreSnapshot<'_>,
    ids: &HashSet<&str>,
    budget: &mut Budget,
) -> StoreResult<Vec<CanonicalAthlete>> {
    let subjects: HashMap<String, CanonicalAthlete> =
        selected(snapshot, Table::Athletes, ids, budget)?;
    let keys: HashSet<_> = subjects.values().map(key).collect();
    drop(subjects);
    let mut rows = Vec::new();
    snapshot.for_each_merged(Table::Athletes, |row: CanonicalAthlete| {
        if keys.contains(&key(&row)) {
            budget.charge(&row)?;
            rows.push(row);
        }
        Ok(())
    })?;
    snapshot.enrich_athletes(&mut rows)?;
    budget.charge(&rows)?;
    Ok(rows)
}
