use std::fs;

use super::{IntegrityReport, IntegrityTable};
use crate::{StorageMode, Store, StoreError, StoreResult, Table, TableWalk};

impl Store {
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
                details: mode_details(self, table, walk)?,
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
                let name = match path.file_name().map(|n| n.to_string_lossy().to_string()) {
                    Some(value) => value,
                    None => path.display().to_string(),
                };
                unreadable.push(name);
            }
        }
        Ok(())
    }
}

fn mode_details(store: &Store, table: Table, walk: TableWalk) -> StoreResult<Vec<String>> {
    let mut details = Vec::new();
    match table.storage_mode() {
        StorageMode::ObservationLog => {
            let keys = match walk.highest_sequence {
                Some(highest) => highest.checked_add(1).ok_or(StoreError::CounterOverflow)?,
                None => 0,
            };
            let mark = store.sequences.next_sequence(table)?;
            if mark < keys {
                details.push(format!(
                    "sequence mark {mark} is behind the highest key stored ({keys})"
                ));
            }
        }
        StorageMode::DerivedGeneration => {
            if walk.repeated_ids > 0 {
                details.push(format!(
                    "{} ids own more than one row of a generation table",
                    walk.repeated_ids
                ));
            }
        }
        StorageMode::DerivedMap => {
            if walk.foreign_sequences > 0 {
                details.push(format!(
                    "{} rows are keyed under a sequence other than zero",
                    walk.foreign_sequences
                ));
            }
            if walk.repeated_ids > 0 {
                details.push(format!("{} ids own more than one row", walk.repeated_ids));
            }
        }
    }
    Ok(details)
}
