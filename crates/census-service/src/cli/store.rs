use anyhow::{ensure, Context, Result};
use census_store::Store;
use clap::Args;
use std::path::{Path, PathBuf};

pub(super) fn print_store_stats(store: &Store) -> Result<()> {
    let stats = store.stats().context("reading the Fjall store stats")?;
    let format = store.store_format().context("reading the store format")?;
    println!("store\t{}", store.root().display());
    println!(
        "schema_version\t{}",
        format
            .schema_version
            .map_or_else(|| "unversioned".to_string(), |version| version.to_string())
    );
    println!(
        "key_format\t{}",
        format
            .key_format
            .map_or_else(|| "unversioned".to_string(), |version| version.to_string())
    );
    if let Some(target) = format.migration_target {
        println!("migration_target\t{target}");
    }
    if let Some(created_by) = &format.created_by {
        println!("created_by\t{created_by}");
    }
    if let Some(created_at) = &format.created_at {
        println!("created_at\t{created_at}");
    }
    println!("evidence_generation\t{}", store.evidence_generation());
    println!("derived_generation\t{}", store.derived_generation());
    for (table, observations) in &stats.tables {
        println!("{table}\t{observations}");
    }
    println!("observations\t{}", stats.observations);
    println!("bytes_on_disk\t{}", stats.bytes_on_disk);
    println!("store_bytes\t{}", stats.store_bytes);
    Ok(())
}

pub(super) fn run_migrate(root: &Path) -> Result<()> {
    let report = Store::migrate(root).context("migrating the store to the current schema")?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
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
    println!("links\t{}", report.links);
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
    println!("links\t{}", report.links);
    for (table, count) in &report.tables {
        println!("table\t{table}\t{count}");
    }
    Ok(())
}

pub(super) fn run_integrity(store: &Store) -> Result<()> {
    let report = store.integrity().context("checking store integrity")?;
    println!("ok\t{}", report.ok);
    report.tables.iter().for_each(|t| {
        let status = if t.expected == t.actual && t.details.is_empty() {
            "ok"
        } else {
            "mismatch"
        };
        println!(
            "table\t{}\texpected={}\tactual={}\t{status}",
            t.table, t.expected, t.actual
        );
        t.details
            .iter()
            .for_each(|detail| println!("detail\t{}\t{detail}", t.table));
    });
    report
        .unreadable_journals
        .iter()
        .for_each(|journal| println!("unreadable_journal\t{journal}"));
    report
        .unreadable_entity_logs
        .iter()
        .for_each(|entity| println!("unreadable_entity_log\t{entity}"));
    report.corruptions.iter().for_each(|corruption| {
        println!(
            "corruption\t{:?}\t{}\t{}",
            corruption.class, corruption.subject, corruption.detail
        );
    });
    report
        .decode_limits
        .iter()
        .for_each(|limit| println!("decode_limit\t{limit}"));
    let cache_issues = store
        .http_cache_integrity()
        .context("checking http cache integrity")?;
    cache_issues
        .iter()
        .for_each(|issue| println!("http_cache\t{issue}"));
    println!("http_cache_ok\t{}", cache_issues.is_empty());
    ensure!(
        report.ok && cache_issues.is_empty(),
        "store integrity check failed: store_ok={}, http_cache_issues={}",
        report.ok,
        cache_issues.len()
    );
    Ok(())
}
