use census_store::{StorageMode, Store, Table};
use std::collections::HashMap;

#[test]
#[ignore = "operator instrument: needs WALK_ROOT naming a store root"]
fn walk_derived_tables() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::var("WALK_ROOT")?;
    let store = Store::open(std::path::Path::new(&root))?;
    let stats = store.stats()?;
    let ledger: HashMap<&str, u64> = stats
        .tables
        .iter()
        .map(|(table, rows)| (table.as_str(), *rows))
        .collect();
    for table in Table::ALL {
        let mode = if table.storage_mode() == StorageMode::ObservationLog {
            "log"
        } else {
            "derived"
        };
        let walk = store.walk_table(table)?;
        println!(
            "WALK\t{}\tmode={mode}\trows={}\tforeign={}\trepeated={}\thighest={:?}\tledger={}",
            table.file(),
            walk.rows,
            walk.foreign_sequences,
            walk.repeated_ids,
            walk.highest_sequence,
            ledger.get(table.file()).copied().map_or(0, |value| value),
        );
    }
    Ok(())
}
