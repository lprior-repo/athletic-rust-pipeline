use super::{IntegrityTable, Store, StoreError, StoreResult};
use crate::StorageMode;
use crate::Table;
pub(super) fn check(store: &Store, tables: &mut Vec<IntegrityTable>) -> StoreResult<()> {
    for table in Table::ALL {
        let walk = store.walk_table(table)?;
        tables.push(IntegrityTable {
            table: table.file().to_string(),
            expected: store.count(table)?,
            actual: walk.rows,
            details: mode_details(store, table, walk)?,
        });
    }
    Ok(())
}

fn mode_details(store: &Store, table: Table, walk: crate::TableWalk) -> StoreResult<Vec<String>> {
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
