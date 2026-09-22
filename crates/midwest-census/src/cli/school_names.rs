//! Extract distinct, trimmed school names for a given state from the entities JSONL.
//!
//! Input : `<store>/entities/schools.jsonl`
//! Output: a sorted, deduplicated, newline-delimited list of school names.
//!
//! Usage: `midwest-census <global-args> school-names --state <ST> --out <PATH>`
//!
//! Replicates the inline `python3 -` heredoc that runs before the `ohsaa` provider in
//! `run_pipeline.sh`.

use anyhow::{Context, Result};
use clap::Args;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

/// Arguments for the `school-names` subcommand.
#[derive(Debug, Args)]
pub(super) struct SchoolNamesArgs {
    /// US jurisdiction code to filter school names (e.g. `OH`).
    #[arg(long)]
    pub(super) state: String,

    /// Path to write the sorted, deduplicated school name list.
    #[arg(long)]
    pub(super) out: PathBuf,
}

/// Run the school-names subcommand.
pub(super) fn run_school_names(
    store: &midwest_census::store::Store,
    args: &SchoolNamesArgs,
) -> Result<()> {
    let schools_path = store.root().join("entities").join("schools.jsonl");
    let p = schools_path.display();

    let mut names: BTreeSet<String> = BTreeSet::new();
    let file = File::open(&schools_path).with_context(|| format!("opening {p}"))?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.with_context(|| format!("reading line from {p}"))?;
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        let record: Value = serde_json::from_str(&line)
            .with_context(|| format!("parsing school record from {p}"))?;
        let state = record.get("state").and_then(|v| v.as_str()).unwrap_or("");
        if state.to_uppercase() == args.state {
            if let Some(name_val) = record.get("name") {
                if let Some(name) = name_val.as_str() {
                    let trimmed = name.trim().to_string();
                    if !trimmed.is_empty() {
                        names.insert(trimmed);
                    }
                }
            }
        }
    }

    let names_count = names.len();
    let out_dir = args
        .out
        .parent()
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from("."));
    let od = out_dir.display();
    fs::create_dir_all(&out_dir).with_context(|| format!("creating output dir {od}"))?;

    let mut out = File::create(&args.out)
        .with_context(|| format!("creating output file {}", args.out.display()))?;
    for name in &names {
        writeln!(out, "{name}")?;
    }

    println!(
        "wrote {names_count} {} names to {}",
        args.state,
        args.out.display()
    );
    Ok(())
}
