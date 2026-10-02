use std::collections::BTreeMap;

use census_domain::model::{RetainedConflict, ReviewCase, ReviewState};
use census_store::{Store, StoreResult, Table};

use crate::review_budget::{encoded_size, Budget};
use crate::{ReviewFamily, ReviewOptions, CHECKPOINT_CASES};

pub(super) struct Intake {
    pub(super) cases: Vec<(ReviewCase, ReviewFamily)>,
    pub(super) cursor: Option<String>,
}

pub(super) fn next(
    store: &Store,
    options: &ReviewOptions,
    after: Option<&str>,
) -> StoreResult<Intake> {
    let snapshot = store.snapshot();
    let mut rows = BTreeMap::new();
    let mut bytes = 0_usize;
    snapshot.for_each_merged(Table::ReviewCases, |case: ReviewCase| {
        if actionable(&case, options) {
            admit(&mut rows, &mut bytes, case, after)?;
        }
        Ok(())
    })?;
    snapshot.for_each_merged(Table::Conflicts, |conflict: RetainedConflict| {
        let case = ReviewCase::pending(
            &conflict.family,
            conflict.subject_id.as_str(),
            conflict.subject.as_str(),
            conflict.detail.as_str(),
        );
        if actionable(&case, options) {
            admit(&mut rows, &mut bytes, case, after)?;
        }
        Ok(())
    })?;
    let cursor = rows.last_key_value().map(|(id, _)| id.clone());
    snapshot.for_each_merged(Table::ReviewCases, |case: ReviewCase| {
        if rows.contains_key(&case.id) {
            if actionable(&case, options) {
                rows.insert(case.id.clone(), case);
            } else {
                rows.remove(&case.id);
            }
        }
        Ok(())
    })?;
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
