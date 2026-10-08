use fjall::{Keyspace, Readable, Snapshot};

use crate::{Receipt, Store, StoreError, StoreResult, MAX_ROWS_PER_TABLE};

impl Store {
    pub fn for_each_receipt(
        &self,
        max: usize,
        visit: impl FnMut(Receipt) -> StoreResult<()>,
    ) -> StoreResult<usize> {
        visit_receipts(&self.db.snapshot(), &self.receipts, max, visit)
    }
}

pub(crate) fn visit_receipts(
    snapshot: &Snapshot,
    receipts: &Keyspace,
    max: usize,
    mut visit: impl FnMut(Receipt) -> StoreResult<()>,
) -> StoreResult<usize> {
    let ceiling = usize::try_from(MAX_ROWS_PER_TABLE).map_err(|_| StoreError::CounterOverflow)?;
    let max = max.min(ceiling);
    let scan = max.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    let mut count = 0_usize;
    for guard in snapshot.iter(receipts).take(scan) {
        if count == max {
            return Err(StoreError::TooManyRows {
                table: "receipts".into(),
                max,
            });
        }
        let (key, value) = guard
            .into_inner()
            .map_err(|source| StoreError::Read { source })?;
        let receipt: Receipt =
            serde_json::from_slice(&value).map_err(|source| StoreError::Decode {
                key: format!("receipts/{}", String::from_utf8_lossy(&key)),
                source,
            })?;
        if key.as_ref() != receipt.operation.as_bytes() {
            return Err(StoreError::Invariant {
                detail: "receipt key differs from its operation identity".into(),
            });
        }
        visit(receipt)?;
        count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    }
    Ok(count)
}
