use super::{bounded, ReadBudget, ReadCounts, StoreSnapshot};
use crate::{StoreError, StoreResult, MAX_OPERATION_BYTES};
use fjall::Readable;

impl StoreSnapshot<'_> {
    #[cfg(test)]
    pub(crate) fn visit_attachment_receipts_bounded(
        &self,
        prefix: &[u8],
        budget: ReadBudget,
        mut visit: impl FnMut(&[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        self.visit_attachment_receipts_with_admission(
            prefix,
            budget,
            &mut (),
            |_| Ok(()),
            |_, key, raw| visit(key, raw),
        )
    }

    pub(crate) fn visit_attachment_receipts_with_admission<G>(
        &self,
        prefix: &[u8],
        budget: ReadBudget,
        state: &mut G,
        admit_row: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        scan_receipts(
            &self.snapshot,
            self.receipts,
            prefix,
            budget,
            state,
            admit_row,
            visit,
        )
    }
}

pub(crate) fn scan_receipts<G>(
    snapshot: &fjall::Snapshot,
    receipts: &fjall::Keyspace,
    prefix: &[u8],
    budget: ReadBudget,
    state: &mut G,
    mut admit_row: impl FnMut(&mut G) -> StoreResult<()>,
    mut visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
) -> StoreResult<ReadCounts> {
    let mut scan = ReceiptScan::new(prefix, budget)?;
    for guard in snapshot.prefix(receipts, prefix) {
        admit_row(state)?;
        scan.charge_visit()?;
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let raw = scan.read(
            &key,
            || {
                snapshot
                    .size_of(receipts, &key)
                    .map_err(|source| StoreError::Read { source })
            },
            || {
                snapshot
                    .get(receipts, &key)
                    .map_err(|source| StoreError::Read { source })
            },
        )?;
        visit(state, &key, &raw)?;
    }
    Ok(scan.finish())
}

pub(crate) struct ReceiptScan {
    counts: ReadCounts,
    budget: ReadBudget,
}

impl ReceiptScan {
    pub(crate) fn new(prefix: &[u8], budget: ReadBudget) -> StoreResult<Self> {
        if prefix.is_empty() || prefix.len() > MAX_OPERATION_BYTES {
            return Err(StoreError::Invariant {
                detail: "invalid attachment receipt prefix".into(),
            });
        }
        Ok(Self {
            counts: ReadCounts::default(),
            budget,
        })
    }

    pub(crate) fn charge_visit(&mut self) -> StoreResult<()> {
        self.counts.key_visits = bounded::charge(
            self.counts.key_visits,
            1,
            self.budget.rows,
            "attachment receipt rows",
        )?;
        Ok(())
    }

    pub(crate) fn read<T: AsRef<[u8]>>(
        &mut self,
        key: &[u8],
        size: impl FnOnce() -> StoreResult<Option<u32>>,
        load: impl FnOnce() -> StoreResult<Option<T>>,
    ) -> StoreResult<T> {
        if key.len() > MAX_OPERATION_BYTES {
            return Err(StoreError::RowTooLarge {
                bytes: key.len(),
                max: MAX_OPERATION_BYTES,
            });
        }
        let size = size()?;
        bounded::read_admitted(key.len(), size, &mut self.counts, self.budget, load)
    }

    pub(crate) fn finish(self) -> ReadCounts {
        self.counts
    }
}
