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
        mut admit_row: impl FnMut(&mut G) -> StoreResult<()>,
        mut visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        if prefix.is_empty() || prefix.len() > MAX_OPERATION_BYTES {
            return Err(StoreError::Invariant {
                detail: "invalid attachment receipt prefix".into(),
            });
        }
        let mut counts = ReadCounts::default();
        for guard in self.snapshot.prefix(self.receipts, prefix) {
            admit_row(state)?;
            counts.key_visits =
                bounded::charge(counts.key_visits, 1, budget.rows, "attachment receipt rows")?;
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            if key.len() > MAX_OPERATION_BYTES {
                return Err(StoreError::RowTooLarge {
                    bytes: key.len(),
                    max: MAX_OPERATION_BYTES,
                });
            }
            let size = self
                .snapshot
                .size_of(self.receipts, &key)
                .map_err(|source| StoreError::Read { source })?;
            let raw = bounded::read_admitted(key.len(), size, &mut counts, budget, || {
                self.snapshot
                    .get(self.receipts, &key)
                    .map_err(|source| StoreError::Read { source })
            })?;
            visit(state, &key, &raw)?;
        }
        Ok(counts)
    }
}
