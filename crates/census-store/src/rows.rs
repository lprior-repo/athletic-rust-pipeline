//! The row ledger: how many rows each table holds, and the walk that establishes a count nobody has.
//!
//! A sequence counter is a pointer, not a count. An append reserves the sequences of a batch before
//! the batch commits, so a commit that fails leaves the counter ahead of the rows the store holds; a
//! derived table spends no sequence at all, so its counter says nothing about its size. What every
//! table does have is rows, and that number is what a status report prints and what
//! [`Store::integrity`](crate::Store::integrity) compares a walk against — so each table keeps
//! a durable count in `meta`, written in the same batch as the rows it counts. Two batches cannot
//! disagree with the keyspace: a batch is applied whole, or not at all.
//!
//! A store written before the ledger existed has no counts, and the walk that derives them happens
//! once, at open, after the legacy import that writes the rows of an older store for the first time.

use fjall::{Keyspace, OwnedWriteBatch, PersistMode};

use super::keys::{split_observation_key, table_prefix, DERIVED_SEQUENCE};
use super::{Store, StoreError, StoreResult, Table};

/// The `meta` row holding one table's row count: `rows:<table>`, beside the `sequence:<table>` mark.
fn row_mark_key(table: Table) -> String {
    format!("rows:{}", table.file())
}

/// What one walk of a table's keys found: the figures its mode's invariant is stated in.
///
/// A log is checked against its sequence mark — the highest key it holds is the mark minus one — and a
/// derived table against its own shape: every row keyed under sequence zero, one row per id. Both are
/// read off one pass of the table, which is what [`Store::walk_table`] is for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TableWalk {
    /// Rows the table holds on disk: one key per row, before the merge a read applies.
    pub rows: u64,
    /// Highest sequence any row is keyed under, or `None` for a table that holds nothing.
    pub highest_sequence: Option<u64>,
    /// Rows keyed under a sequence other than zero. A derived write keys every row under sequence
    /// zero, so a row here is a copy the write should have replaced — and a read merges later keys
    /// over earlier ones, so the copy would win.
    pub foreign_sequences: u64,
    /// Ids owning more than one row. An append-only table expects this — one row per observation of an
    /// id — while a derived table keys one row per id, so a repeat is a stale copy a read folds into
    /// the row that should have replaced it.
    pub repeated_ids: u64,
}

impl Store {
    /// Walk `table` and read off what its keys show, for the mode's own invariant.
    ///
    /// This is the check a `DerivedMap` table is asserted against — one physical row per entity key,
    /// every row keyed under sequence zero, no id owning two rows — and the walk an append-only table's
    /// counter is compared against. One pass, bounded by the table's rows, which is the cost of any
    /// answer here: a key's own sequence is only knowable by reading the key.
    pub fn walk_table(&self, table: Table) -> StoreResult<TableWalk> {
        walk_keys(&self.entities, table)
    }

    /// How many rows `table` holds.
    ///
    /// The count a table keeps is the quantity its writer actually grows: for an append-only table the
    /// observations it has committed, and for a derived table the rows it currently materializes. It
    /// is a *count* rather than the sequence pointer — [`Counters::next_sequence`] is the pointer, and
    /// a batch whose commit failed leaves it ahead of the rows the store holds.
    ///
    /// A store that predates the ledger has no count for a table, and then this walks the table.
    /// [`Store::seed_row_marks`] gives every table a count at open, so the walk answers for a store
    /// that nobody has opened since the ledger landed.
    ///
    /// [`Counters::next_sequence`]: super::sequences::Counters::next_sequence
    pub(super) fn count(&self, table: Table) -> StoreResult<u64> {
        match stored_row_mark(&self.meta, table)? {
            Some(rows) => Ok(rows),
            None => count_rows(&self.entities, table),
        }
    }

    /// Store `rows` as `table`'s count, inside the batch that writes the rows it counts.
    ///
    /// The count rides the batch for the same reason a sequence mark does: a batch is applied whole or
    /// not at all, so a count in a batch of its own could describe rows that were never written, or
    /// miss rows that were.
    pub(super) fn put_row_mark(&self, batch: &mut OwnedWriteBatch, table: Table, rows: u64) {
        batch.insert(&self.meta, row_mark_key(table), rows.to_string().as_bytes());
    }

    /// Give every table a count, one walk per table that has none.
    ///
    /// An open runs this once, after the legacy import, so the walk happens on the open that first
    /// meets a store without counts and never again. Before the import would be wrong: the import
    /// writes rows, and a count derived before it would count what was there before them.
    pub(super) fn seed_row_marks(&self) -> StoreResult<()> {
        let mut derived: Vec<(Table, u64)> = Vec::new();
        for table in Table::ALL {
            if stored_row_mark(&self.meta, table)?.is_none() {
                derived.push((table, count_rows(&self.entities, table)?));
            }
        }
        if derived.is_empty() {
            return Ok(());
        }
        let mut batch = self.db.batch();
        for (table, rows) in &derived {
            self.put_row_mark(&mut batch, *table, *rows);
        }
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }
}

/// One table's count as the store holds it, or `None` for a table the ledger has not reached.
fn stored_row_mark(meta: &Keyspace, table: Table) -> StoreResult<Option<u64>> {
    let key = row_mark_key(table);
    let Some(value) = meta
        .get(&key)
        .map_err(|source| StoreError::Read { source })?
    else {
        return Ok(None);
    };
    match std::str::from_utf8(&value)
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
    {
        Some(rows) => Ok(Some(rows)),
        None => Err(StoreError::Invariant {
            detail: format!("{key} is not a row count"),
        }),
    }
}

/// One table's rows as the store's own key encoding counts them.
pub(super) fn count_rows(entities: &Keyspace, table: Table) -> StoreResult<u64> {
    Ok(walk_keys(entities, table)?.rows)
}

/// Walk every key under `table`'s prefix and read off what they show.
///
/// One pass, bounded by the table's rows: the keys sort by id, so a repeat shows up against the id
/// read just before, and nothing but that id has to be remembered. A key the table's own encoding
/// cannot parse is corruption, not a row to skip past.
fn walk_keys(entities: &Keyspace, table: Table) -> StoreResult<TableWalk> {
    let mut walk = TableWalk::default();
    let mut previous: Vec<u8> = Vec::new();
    for guard in entities.prefix(table_prefix(table)) {
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let (_, id, sequence) =
            split_observation_key(&key).ok_or_else(|| StoreError::Invariant {
                detail: format!("table {} holds a malformed observation key", table.file()),
            })?;
        walk.record(id, sequence, &mut previous);
    }
    Ok(walk)
}

impl TableWalk {
    /// Fold one key into the walk: its row, its sequence, and the id it names.
    ///
    /// The keys of one id sort together, so a repeat is caught against the id read just before it and
    /// nothing but that one id has to be remembered. `previous` is cleared and refilled rather than
    /// reassigned, so the walk allocates nothing per row.
    fn record(&mut self, id: &[u8], sequence: u64, previous: &mut Vec<u8>) {
        self.rows = self.rows.saturating_add(1);
        self.highest_sequence = Some(match self.highest_sequence {
            Some(highest) => highest.max(sequence),
            None => sequence,
        });
        if sequence != DERIVED_SEQUENCE {
            self.foreign_sequences = self.foreign_sequences.saturating_add(1);
        }
        if *previous == id {
            self.repeated_ids = self.repeated_ids.saturating_add(1);
        } else {
            previous.clear();
            previous.extend_from_slice(id);
        }
    }
}
