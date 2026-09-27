use anyhow::{Context, Result};
use census_store::{Store, Table};
use clap::Args;
use std::path::{Path, PathBuf};

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

pub(super) fn run_legacy_import(store: &Store) -> Result<()> {
    let imported = store
        .import_legacy()
        .context("importing the pre-Fjall JSONL journals")?;
    println!("imported\t{}\tobservations", imported.observations);
    println!("skipped\t{}\tderived journals", imported.skipped);
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

fn legacy_journal_bytes(path: &Path) -> Result<Option<u64>> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(Some(metadata.len())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
    }
}

#[derive(Debug, Args)]
#[command(about = "Backup arguments")]
pub(super) struct BackupArgs {
    #[arg(help = "Directory to write the backup into")]
    #[arg(long)]
    pub to: PathBuf,
}

#[derive(Debug, Args)]
#[command(about = "Restore arguments")]
pub(super) struct RestoreArgs {
    #[arg(help = "Directory containing a backup to restore from")]
    #[arg(long)]
    pub from: PathBuf,
    #[arg(help = "Directory to write the restored store into")]
    #[arg(long)]
    pub to: PathBuf,
}

pub(super) fn run_backup(root: &Path, args: &BackupArgs) -> Result<()> {
    let report = Store::backup(root, &args.to).context("backing up the store")?;
    println!("backup\t{}", report.to);
    println!("files\t{}", report.files);
    println!("bytes\t{}", report.bytes);
    println!("elapsed_ms\t{}", report.elapsed_ms);
    for (table, count) in &report.tables {
        println!("table\t{table}\t{count}");
    }
    Ok(())
}

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

pub(super) fn run_integrity(store: &Store) -> Result<()> {
    let report = store.integrity().context("checking store integrity")?;
    println!("ok\t{}", report.ok);
    for t in &report.tables {
        let status = if t.expected == t.actual && t.details.is_empty() {
            "ok"
        } else {
            "mismatch"
        };
        println!(
            "table\t{}\texpected={}\tactual={}\t{status}",
            t.table, t.expected, t.actual
        );
        for detail in &t.details {
            println!("detail\t{}\t{detail}", t.table);
        }
    }
    for j in &report.unreadable_journals {
        println!("unreadable_journal\t{j}");
    }
    for e in &report.unreadable_entity_logs {
        println!("unreadable_entity_log\t{e}");
    }
    Ok(())
}
