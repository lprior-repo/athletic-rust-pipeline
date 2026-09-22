//! Store integrity checking.

use std::fs;

use super::{IntegrityReport, IntegrityTable};
use crate::store::{Store, StoreError, StoreResult, Table};

impl Store {
    /// Check the store's own integrity.
    ///
    /// For every table, counts rows in the Fjall entities keyspace and
    /// compares against the sequence counter.  Checks journal files
    /// are readable.  Reports unreadable entity logs.  Returns ok
    /// only when the mismatch list is empty.
    pub fn integrity(&self) -> StoreResult<IntegrityReport> {
        let mut tables = Vec::with_capacity(Table::ALL.len());
        let mut unreadable_journals = Vec::new();
        let mut unreadable_entity_logs = Vec::new();
        self.check_table_integrity(&mut tables, &mut unreadable_entity_logs)?;
        self.check_journal_integrity(&mut unreadable_journals)?;
        let mismatches: Vec<_> = tables.iter().filter(|t| t.expected != t.actual).collect();
        Ok(IntegrityReport {
            ok: mismatches.is_empty()
                && unreadable_journals.is_empty()
                && unreadable_entity_logs.is_empty(),
            tables,
            unreadable_journals,
            unreadable_entity_logs,
        })
    }

    fn check_table_integrity(
        &self,
        tables: &mut Vec<IntegrityTable>,
        _unreadable: &mut Vec<String>,
    ) -> StoreResult<()> {
        for table in Table::ALL {
            let expected = self.sequences.appended(table);
            let actual = self.fjall_row_count(table)?;
            tables.push(IntegrityTable {
                table: table.file().to_string(),
                expected,
                actual,
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

    /// Count rows in the Fjall entities keyspace for a table.
    fn fjall_row_count(&self, table: Table) -> StoreResult<u64> {
        let prefix = table_prefix(table);
        let mut count: u64 = 0;
        for guard in self.entities.prefix(&prefix) {
            let _ = guard.key().map_err(|source| StoreError::Read { source })?;
            count = count.saturating_add(1);
        }
        Ok(count)
    }
}

/// Build the prefix bytes for one table's entity key prefix.
fn table_prefix(table: Table) -> Vec<u8> {
    let mut out = Vec::with_capacity(table.file().len().saturating_add(1));
    out.extend_from_slice(table.file().as_bytes());
    out.push(0);
    out
}
