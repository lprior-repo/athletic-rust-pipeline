use std::collections::BTreeMap;
use std::ops::ControlFlow;

use census_domain::model::{ReviewCase, ReviewState};
use census_store::{Store, StoreResult, Table};

use crate::review_budget::{encoded_size, Budget};
use crate::{ReviewFamily, ReviewOptions, CHECKPOINT_CASES};

#[path = "review_intake_index.rs"]
mod index;
pub(super) use index::CorpusIndex;

pub(super) struct Intake {
    pub(super) cases: Vec<(ReviewCase, ReviewFamily)>,
    pub(super) cursor: Option<String>,
}

pub(super) fn next(
    store: &Store,
    index: &mut CorpusIndex<'_, '_>,
    options: &ReviewOptions,
    after: Option<&str>,
) -> StoreResult<Intake> {
    let snapshot = store.snapshot();
    let mut rows = BTreeMap::new();
    let mut bytes = 0_usize;
    snapshot.for_each_merged_after(Table::ReviewCases, after, |case: ReviewCase| {
        #[cfg(test)]
        tests::decoded()?;
        if actionable(&case, options) {
            admit(&mut rows, &mut bytes, case, after)?;
        }
        Ok(if rows.len() == CHECKPOINT_CASES {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        })
    })?;
    index.prepare()?;
    for (id, conflict_id) in index.conflicts_after(after)? {
        if rows.len() == CHECKPOINT_CASES
            && rows.last_key_value().is_some_and(|(last, _)| id >= last)
        {
            break;
        }
        let case = index.conflict_case(conflict_id)?;
        if actionable(&case, options) {
            admit(&mut rows, &mut bytes, case, after)?;
        }
    }
    let cursor = rows.last_key_value().map(|(id, _)| id.clone());
    let ids: Vec<_> = rows.keys().cloned().collect();
    for id in ids {
        if let Some(case) = snapshot.get_merged::<ReviewCase>(Table::ReviewCases, &id)? {
            #[cfg(test)]
            tests::decoded()?;
            if actionable(&case, options) {
                rows.insert(id, case);
            } else {
                rows.remove(&id);
            }
        }
    }
    finish(rows, cursor)
}

fn finish(rows: BTreeMap<String, ReviewCase>, cursor: Option<String>) -> StoreResult<Intake> {
    let mut budget = Budget::default();
    let cases = rows
        .into_values()
        .map(|case| {
            budget.charge(&case)?;
            let family = ReviewFamily::from_label(&case.family)
                .ok_or_else(|| crate::consensus::invariant("selected review family disappeared"))?;
            Ok((case, family))
        })
        .collect::<StoreResult<_>>()?;
    Ok(Intake { cases, cursor })
}

fn actionable(case: &ReviewCase, options: &ReviewOptions) -> bool {
    !matches!(case.state, ReviewState::Resolved | ReviewState::Superseded)
        && ReviewFamily::from_label(&case.family)
            .is_some_and(|family| options.families.contains(&family))
}

fn admit(
    rows: &mut BTreeMap<String, ReviewCase>,
    bytes: &mut usize,
    case: ReviewCase,
    after: Option<&str>,
) -> StoreResult<()> {
    if after.is_some_and(|after| case.id.as_str() <= after) || rows.contains_key(&case.id) {
        return Ok(());
    }
    if rows.len() == CHECKPOINT_CASES {
        if rows.last_key_value().is_some_and(|(id, _)| case.id >= *id) {
            return Ok(());
        }
        if let Some((_, removed)) = rows.pop_last() {
            *bytes = bytes
                .checked_sub(encoded_size(&removed)?)
                .ok_or(census_store::StoreError::CounterOverflow)?;
        }
    }
    let size = encoded_size(&case)?;
    let mut budget = Budget::default();
    budget.add(*bytes)?;
    budget.add(size)?;
    *bytes = bytes
        .checked_add(size)
        .ok_or(census_store::StoreError::CounterOverflow)?;
    rows.insert(case.id.clone(), case);
    Ok(())
}

#[cfg(test)]
pub(super) fn decoded() -> StoreResult<()> {
    tests::decoded()
}

#[cfg(test)]
#[path = "review_intake_tests.rs"]
mod tests;
