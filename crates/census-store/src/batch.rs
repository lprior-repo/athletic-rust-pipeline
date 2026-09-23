//! The mechanics a write shares: the rows a derived batch stages and clears, and the ceilings that
//! refuse a batch or a journal entry before it exists.
//!
//! `write.rs` holds the store's write API; the pieces those methods share live here. The ceilings are
//! enforced on the write side because that is the only place they can be: a committed batch past one
//! leaves a table no later scan can read, and nothing short of deleting rows repairs that.

use fjall::{Keyspace, OwnedWriteBatch};
use serde::Serialize;
use std::collections::HashSet;

use super::keys::{
    id_prefix, observation_id, observation_key, split_observation_key, table_prefix,
    DERIVED_SEQUENCE,
};
use super::{StorageMode, StoreError, StoreResult, Table, MAX_ROWS_PER_TABLE};

/// Refuse a batch that would leave `table` holding more than `MAX_ROWS_PER_TABLE` rows.
///
/// The ceiling is what keeps the store's reads bounded — [`Store::scan`](crate::Store::scan)
/// aborts on the row past it — so the writer holds it for the reader: a committed batch past the
/// ceiling leaves a table no later scan can read, and nothing short of deleting rows repairs that.
/// `rows` is the sequence the batch would reach for a table that spends sequences, and the batch's own
/// row count for one that does not; the two modes are the ceiling applied to the quantity each table
/// actually grows by.
pub(super) fn refuse_over_bound(table: Table, rows: u64) -> StoreResult<()> {
    if rows > MAX_ROWS_PER_TABLE {
        return Err(StoreError::TooManyRows {
            table: table.file().to_string(),
            max: usize::try_from(MAX_ROWS_PER_TABLE).unwrap_or(usize::MAX),
        });
    }
    Ok(())
}

/// How much of a caller-supplied name an error message carries.
const MAX_JOURNAL_LABEL_CHARS: usize = 64;

/// Refuse a journal entry whose key or serialized value is past the ceiling that bounds it.
///
/// The refusal names the phase and the key so the caller that wrote it can be found, and it names them
/// truncated: the entry being refused for its size is exactly the one whose size cannot be reproduced
/// in a message.
pub(super) fn refuse_over_journal(
    phase: &str,
    key: &str,
    what: &'static str,
    bytes: usize,
    max: usize,
) -> StoreResult<()> {
    if bytes > max {
        return Err(StoreError::JournalTooLarge {
            what,
            phase: label(phase),
            key: label(key),
            bytes,
            max,
        });
    }
    Ok(())
}

/// A caller-supplied name, as much of it as an error message can carry.
fn label(name: &str) -> String {
    name.chars().take(MAX_JOURNAL_LABEL_CHARS).collect()
}

/// What a derived batch staged: the ids it names, and how many of them the table did not hold.
pub(super) struct Staged {
    pub(super) named: HashSet<String>,
    pub(super) added: u64,
}

impl Staged {
    /// The rows a batch leaves standing when that batch is all the table holds.
    pub(super) fn named_count(&self) -> StoreResult<u64> {
        u64::try_from(self.named.len()).map_err(|_| StoreError::CounterOverflow)
    }
}

/// Key every record of a derived batch and insert it, replacing the row its id already holds.
///
/// A derived row is one row per id, so an id repeated inside one batch writes the same key twice — the
/// last record wins — and an id the table already holds is replaced where it stands. What a batch adds
/// is therefore the ids it is the first to name, whichever of the two wrote them. The ids a batch is
/// the first to name also have their foreign rows cleared, in the same batch, so a table that took
/// rows from an older store comes back to one row per id as the derivation names them again.
pub(super) fn stage_derived<T: Serialize>(
    batch: &mut OwnedWriteBatch,
    entities: &Keyspace,
    table: Table,
    records: &[T],
) -> StoreResult<Staged> {
    let mut staged = Staged {
        named: HashSet::with_capacity(records.len()),
        added: 0,
    };
    for record in records {
        let value = serde_json::to_vec(record).map_err(|source| StoreError::Json {
            detail: "serializing derived state".to_string(),
            source,
        })?;
        let id = observation_id(&value)?.to_string();
        let key = observation_key(table, &id, DERIVED_SEQUENCE);
        let held = entities
            .get(&key)
            .map_err(|source| StoreError::Read { source })?
            .is_some();
        let first_named = staged.named.insert(id.clone());
        if first_named && !held {
            staged.added = staged.added.saturating_add(1);
        }
        if first_named && table.storage_mode() != StorageMode::ObservationLog {
            drop_foreign(entities, batch, table, &id)?;
        }
        batch.insert(entities, key, value);
    }
    Ok(staged)
}

/// Remove every row of `id` that sits above the derived sequence.
///
/// A derived table is a read model and the derivation is its only legitimate author, so a copy at
/// another sequence is one an older store imported before the gate existed: the merged read takes the
/// later row, which puts that copy *over* the row being written, and a table whose rows are replaced
/// would keep losing to rows it no longer derives. The removal rides in the same batch as the insert,
/// so no reader sees the id with its row gone and the replacement not yet in place. An append-only
/// table holds evidence rather than read models, and nothing here removes evidence.
fn drop_foreign(
    entities: &Keyspace,
    batch: &mut OwnedWriteBatch,
    table: Table,
    id: &str,
) -> StoreResult<()> {
    for guard in entities.prefix(id_prefix(table, id)) {
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let (_, _, sequence) =
            split_observation_key(&key).ok_or_else(|| StoreError::Invariant {
                detail: format!("table {} holds a malformed observation key", table.file()),
            })?;
        if sequence != DERIVED_SEQUENCE {
            batch.remove(entities, key);
        }
    }
    Ok(())
}

/// Remove every row of a snapshot table that the batch does not name.
///
/// A snapshot write describes the table's whole content, so a write that only upserted would keep a row
/// for every subject the newest derivation no longer derives — a jurisdiction that lost its last meet
/// would go on reporting the meet it had. A map write is the opposite: its rows are keyed to findings
/// that persist, so it leaves the rows it does not name standing.
pub(super) fn drop_unnamed(
    entities: &Keyspace,
    batch: &mut OwnedWriteBatch,
    table: Table,
    named: &HashSet<String>,
) -> StoreResult<()> {
    let prefix = table_prefix(table);
    for guard in entities.prefix(&prefix) {
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let (_, id, _) = split_observation_key(&key).ok_or_else(|| StoreError::Invariant {
            detail: format!("table {} holds a malformed observation key", table.file()),
        })?;
        if !named.contains(id) {
            batch.remove(entities, key);
        }
    }
    Ok(())
}
