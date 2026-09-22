//! Store inspection subcommands: per-table observation counts and the one-time import of
//! the pre-Fjall JSONL journals.

use anyhow::{Context, Result};
use midwest_census::store::{Store, Table};
use std::path::Path;

/// `<table>\t<observations>` for every table, then the store's own totals.
pub(super) fn print_store_stats(store: &Store) -> Result<()> {
    let stats = store.stats().context("reading the Fjall store stats")?;
    println!("store\t{}", store.root().display());
    for (table, observations) in &stats.tables {
        println!("{table}\t{observations}");
    }
    println!("observations\t{}", stats.observations);
    println!("bytes_on_disk\t{}", stats.bytes_on_disk);
    println!("store_bytes\t{}", stats.store_bytes);
    Ok(())
}

/// [`Store::open`] performs the one-time pre-Fjall JSONL import before a command sees the store, so
/// this reports the import source it read, then the resulting store. A table is marked imported in
/// the store's `meta` keyspace and is never imported twice; the JSONL files are left in place as the
/// record of what the database was built from.
pub(super) fn run_legacy_import(store: &Store) -> Result<()> {
    for table in Table::ALL {
        let path = store.table_path(table);
        match legacy_journal_bytes(&path)? {
            Some(bytes) => println!(
                "legacy\t{}\t{bytes} bytes\t{}",
                table.file(),
                path.display()
            ),
            None => println!("legacy\t{}\tabsent", table.file()),
        }
    }
    println!(
        "legacy_journal_dir\t{}",
        store.root().join("journal").display()
    );
    print_store_stats(store)
}

/// Size of one legacy entity journal, or `None` when the pre-Fjall file was never written.
fn legacy_journal_bytes(path: &Path) -> Result<Option<u64>> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(Some(metadata.len())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
    }
}
