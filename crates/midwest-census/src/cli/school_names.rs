//! Extract distinct, trimmed school names for a given state from the consolidated schools snapshot.
//!
//! Input : `<store>/out/schools.jsonl` (the consolidated snapshot)
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
use std::fs::File;
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
    let schools_path = store.out_dir().join("schools.jsonl");
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

    // `publish_atomically` creates the destination's directory itself (`store/read/snapshot.rs`), so
    // the place that writes the file is the one place that creates what it needs.
    midwest_census::store::read::publish_atomically(&args.out, |temporary| {
        let mut out =
            File::create(temporary).map_err(|source| midwest_census::store::StoreError::Io {
                path: args.out.clone(),
                source,
            })?;
        for name in &names {
            writeln!(out, "{name}").map_err(|source| midwest_census::store::StoreError::Io {
                path: args.out.clone(),
                source,
            })?;
        }
        out.flush()
            .map_err(|source| midwest_census::store::StoreError::Io {
                path: args.out.clone(),
                source,
            })
    })?;

    println!(
        "wrote {names_count} {} names to {}",
        args.state,
        args.out.display()
    );
    Ok(())
}
