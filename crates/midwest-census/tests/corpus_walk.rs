//! TEMPORARY (StorageModeC self-heal corpus evidence): walks every table of a store root and prints the
//! four facts the repair is judged by, beside the ledger count the store keeps for the same table.
//!
//! Run with the root named, against a copy or the live root:
//! `WALK_ROOT=<dir> cargo test -p midwest-census --test corpus_walk -- --ignored --nocapture`
//!
//! Ignored by default: it is an operator instrument over a root the operator names, and an unset
//! `WALK_ROOT` is not a failing claim about the code. Deleted once the before/after numbers are
//! recorded; nothing else imports it.

use census_store::{StorageMode, Store, Table};
use std::collections::HashMap;

#[test]
#[ignore = "operator instrument: needs WALK_ROOT naming a store root"]
fn walk_derived_tables() {
    let root = std::env::var("WALK_ROOT").expect("WALK_ROOT must name a store root");
    let store = Store::open(std::path::Path::new(&root)).expect("opening the store root");
    let stats = store.stats().expect("reading the store's own counts");
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
        let walk = store.walk_table(table).expect("walking the table");
        println!(
            "WALK\t{}\tmode={mode}\trows={}\tforeign={}\trepeated={}\thighest={:?}\tledger={}",
            table.file(),
            walk.rows,
            walk.foreign_sequences,
            walk.repeated_ids,
            walk.highest_sequence,
            ledger.get(table.file()).copied().unwrap_or(0),
        );
    }
}
