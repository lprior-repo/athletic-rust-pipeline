//! Receipts: the durable record that one logical operation was applied, written beside the rows it
//! names.
//!
//! An append that reaches the database but whose acknowledgement never reaches its caller is
//! replayed by whoever owns the retry. Written as two commits — rows first, receipt second — that
//! replay appends the page a second time, because the writer that should have remembered the first
//! one died before it could. Written as one commit ([`StoreBatch::commit_once`]) the replay finds
//! the receipt standing beside the rows it already wrote, writes nothing, and answers with the
//! receipt the first application left.
//!
//! A receipt is bound to two things, not one: the caller's operation id, and the digest of the
//! payload that operation carried. A repeat under the same id with the same digest is exactly the
//! replay this exists for. A repeat under the same id with a *different* digest is a caller that
//! reused a name for different work, and it fails closed: appending it would put a page under an id
//! that does not describe it, and the next replay would then find a receipt claiming rows the store
//! never received. Neither id nor digest is derived from the other — an id derived from the payload
//! cannot detect a changed payload, which is the failure this refuses.
//!
//! Receipts are never overwritten and never deleted, so the count [`Store::receipt_count`] reports is
//! the number of operations the store has applied, and a row's day is the application that wrote it.

use chrono::NaiveDate;
use fjall::PersistMode;
use serde::{Deserialize, Serialize};

use super::clock::{Clock, SystemClock};
use super::{Store, StoreError, StoreResult};

/// The longest operation id the store keys a receipt under.
///
/// Fjall asserts its keys stay under 64 KiB and asserts on the insert rather than returning an
/// error, so an over-long id is refused here, before the batch holds anything.
pub const MAX_OPERATION_BYTES: usize = 512;

/// The longest payload digest a receipt records.
pub const MAX_DIGEST_BYTES: usize = 256;

/// What one applied operation left behind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// The caller's stable name for the operation.
    pub operation: String,
    /// The digest of the payload the operation carried, computed by the caller.
    pub digest: String,
    /// The day (`YYYY-MM-DD`) the application that wrote this row ran.
    ///
    /// A day, not an instant: the only use a later reader makes of it is choosing which receipts a
    /// retention policy may remove, and a day is what both sides of that comparison can agree on.
    pub at: String,
    /// Rows that application appended.
    pub appended: u64,
}

/// What a receipted commit did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Application {
    /// This commit wrote the rows, the table accounting and the receipt, in one durability
    /// boundary.
    Written(Receipt),
    /// An earlier commit had already applied this operation with this digest. Nothing was written —
    /// no row, no counter, no journal entry — and the receipt is the one that commit left.
    Repeated(Receipt),
}

impl Application {
    /// The rows this call is answerable for: the receipt's count when it wrote them, zero when it
    /// found them already written.
    ///
    /// A replay must report zero, or a caller summing replies counts one page twice — the same
    /// double count the receipt exists to prevent in the store.
    pub fn appended(&self) -> u64 {
        match self {
            Self::Written(receipt) => receipt.appended,
            Self::Repeated(_) => 0,
        }
    }

    /// Whether this call wrote the operation, as opposed to finding it already applied.
    pub fn written(&self) -> bool {
        matches!(self, Self::Written(_))
    }

    /// Whether this call found the operation already applied and wrote nothing.
    pub fn repeated(&self) -> bool {
        matches!(self, Self::Repeated(_))
    }

    /// The receipt behind the outcome, whether this call wrote it or found it.
    pub fn receipt(&self) -> &Receipt {
        match self {
            Self::Written(receipt) | Self::Repeated(receipt) => receipt,
        }
    }
}

/// What a receipted commit does with its operation, decided before anything is written.
pub(super) enum Decision {
    /// No receipt stands under this id: write one with the batch.
    First,
    /// The operation is already applied with this digest: write nothing, answer with this.
    Repeat(Receipt),
}

impl Store {
    /// The receipt one operation left, if the store applied it.
    ///
    /// This is the read side of the write path's own check: an operator repairing a lost
    /// acknowledgement asks the store what it holds, rather than what a caller believes it holds.
    pub fn receipt(&self, operation: &str) -> StoreResult<Option<Receipt>> {
        let raw = self
            .receipts
            .get(operation.as_bytes())
            .map_err(|source| StoreError::Read { source })?;
        let Some(value) = raw else {
            return Ok(None);
        };
        let receipt: Receipt =
            serde_json::from_slice(value.as_ref()).map_err(|source| StoreError::Decode {
                key: format!("receipts/{operation}"),
                source,
            })?;
        Ok(Some(receipt))
    }

    /// How many operations the store holds receipts for.
    ///
    /// Exact rather than an estimate: no path overwrites or deletes a receipt, so the keyspace's
    /// entry count is the number of operations applied.
    pub fn receipt_count(&self) -> StoreResult<u64> {
        let count = self
            .receipts
            .len()
            .map_err(|source| StoreError::Read { source })?;
        u64::try_from(count).map_err(|_| StoreError::CounterOverflow)
    }
}

/// Refuse an operation id or digest the store cannot key or record.
///
/// An empty digest is refused for the same reason an empty id is: every payload would hash to it, so
/// a changed payload would read as a repeat of the operation and be silently dropped.
pub(super) fn refuse_over_operation(operation: &str, digest: &str) -> StoreResult<()> {
    if operation.is_empty() {
        return Err(StoreError::Refused {
            detail: "operation id must not be empty: a receipt keyed by nothing records nothing"
                .to_string(),
        });
    }
    if operation.len() > MAX_OPERATION_BYTES {
        return Err(StoreError::Refused {
            detail: format!(
                "operation id of {} bytes exceeds the {MAX_OPERATION_BYTES}-byte ceiling",
                operation.len()
            ),
        });
    }
    if digest.is_empty() {
        return Err(StoreError::Refused {
            detail: "payload digest must not be empty: an empty digest matches every payload"
                .to_string(),
        });
    }
    if digest.len() > MAX_DIGEST_BYTES {
        return Err(StoreError::Refused {
            detail: format!(
                "payload digest of {} bytes exceeds the {MAX_DIGEST_BYTES}-byte ceiling",
                digest.len()
            ),
        });
    }
    Ok(())
}

/// Decide what a commit offering `operation` does, without writing anything.
///
/// Called inside the append lock, so two batches offering the same operation cannot both read an
/// absent receipt and both write one.
pub(super) fn decide(store: &Store, operation: &str, digest: &str) -> StoreResult<Decision> {
    match store.receipt(operation)? {
        None => Ok(Decision::First),
        Some(receipt) if receipt.digest == digest => Ok(Decision::Repeat(receipt)),
        Some(receipt) => Err(StoreError::Invariant {
            detail: format!(
                "operation {operation} was applied on {} with digest {} and is now offered digest \
                 {digest}: the same id cannot name two payloads",
                receipt.at, receipt.digest
            ),
        }),
    }
}

/// Build the receipt row one first application writes, stamped with the store's own day.
pub(super) fn first(operation: &str, digest: &str, appended: u64) -> Receipt {
    Receipt {
        operation: operation.to_string(),
        digest: digest.to_string(),
        at: SystemClock.today(),
        appended,
    }
}

/// What one [`Store::prune_receipts`] call removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pruned {
    /// Receipts deleted: the operations whose replay window has closed.
    pub removed: u64,
    /// Receipts kept because the store cannot read a day off them, and so cannot show that their
    /// window has closed.
    pub undated: u64,
}

impl Store {
    /// Delete the receipts of operations whose replay window has closed, and report what was kept.
    ///
    /// `before` is a `YYYY-MM-DD` day: every receipt stamped strictly earlier is removed, and every
    /// receipt stamped that day or later stays. The day is the caller's policy — the store holds no
    /// opinion on how long a replay may reach back — and it is safe only while no *retained*
    /// invocation can still replay the operation, which Restate's own journal and idempotency
    /// retention is what bounds.
    ///
    /// The prune is one commit and one critical section of the append lock, taken with every
    /// receipted commit: a prune that raced one could delete the receipt a commit had just written
    /// and leave its rows behind with nothing to recognize them by. A row whose day cannot be read is
    /// kept and counted in [`Pruned::undated`] rather than removed, because deleting it could
    /// re-open the double append it exists to prevent — the blind spot is reported, not acted on.
    ///
    /// Cost is one pass over the receipts keyspace, which holds one small row per applied operation.
    pub fn prune_receipts(&self, before: &str) -> StoreResult<Pruned> {
        let boundary =
            NaiveDate::parse_from_str(before, "%Y-%m-%d").map_err(|_| StoreError::Refused {
                detail: format!("retention boundary {before} is not a YYYY-MM-DD day"),
            })?;
        let _appends = self.lock_appends();
        let mut pruned = Pruned::default();
        let mut doomed: Vec<Vec<u8>> = Vec::new();
        for guard in self.receipts.iter() {
            let (key, raw) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            let receipt: Receipt =
                serde_json::from_slice(raw.as_ref()).map_err(|source| StoreError::Decode {
                    key: format!("receipts/{}", String::from_utf8_lossy(&key)),
                    source,
                })?;
            match NaiveDate::parse_from_str(&receipt.at, "%Y-%m-%d") {
                Ok(stamped) if stamped < boundary => doomed.push(key.to_vec()),
                Ok(_) => {}
                Err(_) => pruned.undated = pruned.undated.saturating_add(1),
            }
        }
        if doomed.is_empty() {
            return Ok(pruned);
        }
        pruned.removed = u64::try_from(doomed.len()).map_err(|_| StoreError::CounterOverflow)?;
        let mut batch = self.db.batch();
        for key in doomed {
            batch.remove(&self.receipts, key);
        }
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(pruned)
    }
}
