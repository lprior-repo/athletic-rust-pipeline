//! Store integrity checking.

use std::fs;

use super::{IntegrityReport, IntegrityTable};
use crate::store::{StorageMode, Store, StoreError, StoreResult, Table, TableWalk};

impl Store {
    /// Check the store's own integrity.
    ///
    /// Every table is walked once and then held to the invariant its own write mode states. Each
    /// table's ledger count — the number its writers keep, stored in the same batch as the rows it
    /// counts — must equal the rows its keyspace holds; a walk that disagrees is drift, and drift is
    /// what the count is kept for. An append-only table's sequence mark may stand *in front* of its
    /// keys, which is exactly what a batch that reserved and never committed leaves, but never behind
    /// them: a reopen would then hand out a sequence the keyspace already holds, and the observation
    /// written under it would overwrite the one already there. A derived table holds one row per id,
    /// every row keyed under sequence zero — a foreign sequence is a copy a read merges *under* the row
    /// that should have replaced it, and a repeated id is a second key a read folds into one entity.
    ///
    /// Checks journal files are readable. Reports unreadable entity logs. Returns ok only when no table
    /// contradicts itself and no such file exists.
    pub fn integrity(&self) -> StoreResult<IntegrityReport> {
        let mut tables = Vec::with_capacity(Table::ALL.len());
        let mut unreadable_journals = Vec::new();
        let mut unreadable_entity_logs = Vec::new();
        self.check_table_integrity(&mut tables, &mut unreadable_entity_logs)?;
        self.check_journal_integrity(&mut unreadable_journals)?;
        let ok = tables
            .iter()
            .all(|table| table.expected == table.actual && table.details.is_empty());
        Ok(IntegrityReport {
            ok: ok && unreadable_journals.is_empty() && unreadable_entity_logs.is_empty(),
            tables,
            unreadable_journals,
            unreadable_entity_logs,
        })
    }

    /// Every table once: its ledger count against the rows its keyspace holds, and its mode's own
    /// invariant as [`Store::walk_table`] reads it.
    fn check_table_integrity(
        &self,
        tables: &mut Vec<IntegrityTable>,
        _unreadable: &mut Vec<String>,
    ) -> StoreResult<()> {
        for table in Table::ALL {
            let walk = self.walk_table(table)?;
            tables.push(IntegrityTable {
                table: table.file().to_string(),
                expected: self.count(table)?,
                actual: walk.rows,
                details: mode_details(self, table, walk),
            });
        }
        Ok(())
    }

    fn check_journal_integrity(&self, unreadable: &mut Vec<String>) -> StoreResult<()> {
        let journal_dir = self.root().join("journal");
        if !journal_dir.exists() {
            return Ok(());
        }
        let entries = fs::read_dir(&journal_dir).map_err(|source| StoreError::Io {
            path: journal_dir.clone(),
            source,
        })?;
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if fs::File::open(&path).is_err() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                unreadable.push(name);
            }
        }
        Ok(())
    }
}

/// The mode-specific facts a count cannot express, in the words the report prints them in.
///
/// A derived table's two facts come from the same walk that counts its rows, so a table is only read
/// once: the counts are the check that its writers kept the ledger, and these are the check that its
/// keys are the shape its mode says they are.
fn mode_details(store: &Store, table: Table, walk: TableWalk) -> Vec<String> {
    let mut details = Vec::new();
    match table.storage_mode() {
        StorageMode::ObservationLog => {
            let keys = walk
                .highest_sequence
                .map_or(0, |highest| highest.saturating_add(1));
            let mark = store.sequences.next_sequence(table);
            if mark < keys {
                details.push(format!(
                    "sequence mark {mark} is behind the highest key stored ({keys})"
                ));
            }
        }
        StorageMode::DerivedSnapshot | StorageMode::DerivedMap => {
            if walk.foreign_sequences > 0 {
                details.push(format!(
                    "{} rows are keyed under a sequence other than zero",
                    walk.foreign_sequences
                ));
            }
            if walk.repeated_ids > 0 {
                details.push(format!(
                    "{} ids own more than one row",
                    walk.repeated_ids
                ));
            }
        }
    }
    details
}
