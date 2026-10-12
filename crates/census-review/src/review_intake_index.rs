use census_domain::model::{RetainedConflict, ReviewCase};
use census_store::{
    StoreError, StoreResult, StoreSnapshot, Table, MAX_ID_BYTES, MAX_ROWS_PER_TABLE,
};

pub(crate) struct CorpusIndex<'a, 's> {
    snapshot: &'a StoreSnapshot<'s>,
    conflicts: Option<Vec<(String, String)>>,
}

impl<'a, 's> CorpusIndex<'a, 's> {
    pub(crate) fn new(snapshot: &'a StoreSnapshot<'s>) -> Self {
        Self {
            snapshot,
            conflicts: None,
        }
    }

    pub(super) fn prepare(&mut self) -> StoreResult<()> {
        if self.conflicts.is_none() {
            self.conflicts = Some(read_conflicts(self.snapshot)?);
        }
        Ok(())
    }

    pub(super) fn conflicts_after(&self, after: Option<&str>) -> StoreResult<&[(String, String)]> {
        let conflicts = self
            .conflicts
            .as_deref()
            .ok_or_else(|| crate::consensus::invariant("conflict index disappeared"))?;
        let start = after.map_or(0, |after| {
            conflicts.partition_point(|(id, _)| id.as_str() <= after)
        });
        conflicts.get(start..).ok_or(StoreError::CounterOverflow)
    }

    pub(super) fn conflict_case(&self, id: &str) -> StoreResult<ReviewCase> {
        let conflict = self
            .snapshot
            .get_merged::<RetainedConflict>(Table::Conflicts, id)?
            .ok_or_else(|| {
                crate::consensus::invariant("indexed conflict disappeared from captured snapshot")
            })?;
        Ok(case(&conflict))
    }
}

pub(super) fn case(conflict: &RetainedConflict) -> ReviewCase {
    ReviewCase::pending(
        &conflict.family,
        &conflict.subject_id,
        &conflict.subject,
        &conflict.detail,
    )
}

fn read_conflicts(snapshot: &StoreSnapshot<'_>) -> StoreResult<Vec<(String, String)>> {
    let mut rows = Vec::new();
    let mut bytes = 0_u64;
    let key_bytes = u64::try_from(MAX_ID_BYTES).map_err(|_| StoreError::CounterOverflow)?;
    let max = key_bytes
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(128))
        .and_then(|bytes| bytes.checked_mul(MAX_ROWS_PER_TABLE))
        .ok_or(StoreError::CounterOverflow)?;
    snapshot.for_each_merged(Table::Conflicts, |conflict: RetainedConflict| {
        #[cfg(test)]
        super::tests::decoded()?;
        let case = case(&conflict);
        let size = case
            .id
            .len()
            .checked_add(conflict.id.len())
            .ok_or(StoreError::CounterOverflow)?;
        bytes = bytes
            .checked_add(u64::try_from(size).map_err(|_| StoreError::CounterOverflow)?)
            .ok_or(StoreError::CounterOverflow)?;
        if bytes > max {
            return Err(crate::consensus::invariant(
                "conflict index exceeds bounded key bytes",
            ));
        }
        rows.try_reserve(1).map_err(|error| {
            crate::consensus::invariant(format!("cannot reserve conflict index: {error}"))
        })?;
        rows.push((case.id, conflict.id));
        Ok(())
    })?;
    rows.sort_unstable();
    Ok(rows)
}
