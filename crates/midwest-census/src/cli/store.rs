//! Store inspection subcommands: per-table observation counts and the one-time import of
//! the pre-Fjall JSONL journals.

use anyhow::{Context, Result};
use clap::Args;
use midwest_census::store::{Store, Table};
use std::path::{Path, PathBuf};

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

/// Backup arguments.
#[derive(Debug, Args)]
pub(super) struct BackupArgs {
    /// Directory to write the backup into.
    #[arg(long)]
    pub to: PathBuf,
}

/// Restore arguments.
#[derive(Debug, Args)]
pub(super) struct RestoreArgs {
    /// Directory containing a backup to restore from.
    #[arg(long)]
    pub from: PathBuf,
    /// Directory to write the restored store into.
    #[arg(long)]
    pub to: PathBuf,
}

/// Backup the store to a directory.
pub(super) fn run_backup(store: &Store, args: &BackupArgs) -> Result<()> {
    let report = store.backup(&args.to).context("backing up the store")?;
    println!("backup\t{}", report.to);
    println!("files\t{}", report.files);
    println!("bytes\t{}", report.bytes);
    println!("elapsed_ms\t{}", report.elapsed_ms);
    for (table, count) in &report.tables {
        println!("table\t{table}\t{count}");
    }
    Ok(())
}

/// Restore a backup into a new directory.
pub(super) fn run_restore(args: &RestoreArgs) -> Result<()> {
    let report = Store::restore(&args.from, &args.to).context("restoring the store")?;
    println!("restored\tfrom {}\tto {}", report.from, report.to);
    println!("files\t{}", report.files);
    println!("bytes\t{}", report.bytes);
    for (table, count) in &report.tables {
        println!("table\t{table}\t{count}");
    }
    Ok(())
}

/// Check the store's integrity.
pub(super) fn run_integrity(store: &Store) -> Result<()> {
    let report = store.integrity().context("checking store integrity")?;
    println!("ok\t{}", report.ok);
    for t in &report.tables {
        let status = if t.expected == t.actual {
            "ok"
        } else {
            "mismatch"
        };
        println!(
            "table\t{}\texpected={}\tactual={}\t{status}",
            t.table, t.expected, t.actual
        );
    }
    for j in &report.unreadable_journals {
        println!("unreadable_journal\t{j}");
    }
    for e in &report.unreadable_entity_logs {
        println!("unreadable_entity_log\t{e}");
    }
    Ok(())
}
