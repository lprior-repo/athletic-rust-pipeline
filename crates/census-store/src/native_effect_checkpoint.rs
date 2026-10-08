use fjall::{Readable, Snapshot};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::keys::Layout;
use crate::table::StorageMode;
use crate::{Receipt, Store, StoreError, StoreResult, Table};

const MAX_NATIVE_CHECKPOINT_ROWS: usize = 100_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEffectRow {
    pub table: Table,
    pub key: Vec<u8>,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEffectCheckpoint {
    pub schema: u8,
    pub sequence: u64,
    pub rows: Vec<NativeEffectRow>,
    pub receipts: Vec<Receipt>,
}

impl Store {
    pub fn native_effect_checkpoint(&self) -> StoreResult<NativeEffectCheckpoint> {
        self.with_native_effect_checkpoint(core::convert::identity)
    }

    pub fn with_native_effect_checkpoint<T>(
        &self,
        witness: impl FnOnce(NativeEffectCheckpoint) -> T,
    ) -> StoreResult<T> {
        let _appends = self.lock_appends();
        let snapshot = self.db.snapshot();
        let layout = Layout {
            derived_generation: self.derived_generation(),
        };
        let mut rows = Vec::new();
        for table in Table::ALL {
            if matches!(table.storage_mode(), StorageMode::ObservationLog) {
                collect_rows(&snapshot, self, layout.prefix(table), table, &mut rows)?;
            }
        }
        let mut receipts = Vec::new();
        crate::receipt::scan::visit_receipts(
            &snapshot,
            &self.receipts,
            MAX_NATIVE_CHECKPOINT_ROWS,
            |receipt| {
                reserve(&mut receipts, "native acknowledged receipts")?;
                receipts.push(receipt);
                Ok(())
            },
        )?;
        Ok(witness(NativeEffectCheckpoint {
            schema: 1,
            sequence: snapshot.seqno(),
            rows,
            receipts,
        }))
    }
}

fn collect_rows(
    snapshot: &Snapshot,
    store: &Store,
    prefix: Vec<u8>,
    table: Table,
    rows: &mut Vec<NativeEffectRow>,
) -> StoreResult<()> {
    let limit = MAX_NATIVE_CHECKPOINT_ROWS
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    for guard in snapshot.prefix(&store.entities, &prefix).take(limit) {
        reserve(rows, "native acknowledged source rows")?;
        let (key, value) = guard
            .into_inner()
            .map_err(|source| StoreError::Read { source })?;
        let mut identity = Vec::new();
        identity
            .try_reserve_exact(key.len())
            .map_err(|_| StoreError::Refused {
                detail: "native source key allocation failed".into(),
            })?;
        identity.extend_from_slice(&key);
        rows.push(NativeEffectRow {
            table,
            key: identity,
            digest: format!("{:x}", Sha256::digest(&value)),
        });
    }
    Ok(())
}

fn reserve<T>(values: &mut Vec<T>, subject: &'static str) -> StoreResult<()> {
    if values.len() >= MAX_NATIVE_CHECKPOINT_ROWS {
        return Err(StoreError::TooManyRows {
            table: subject.into(),
            max: MAX_NATIVE_CHECKPOINT_ROWS,
        });
    }
    values.try_reserve(1).map_err(|_| StoreError::Refused {
        detail: format!("{subject} allocation failed"),
    })
}
