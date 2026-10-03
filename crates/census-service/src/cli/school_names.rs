use anyhow::{Context, Result};
use clap::Args;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Args)]
#[command(about = "Arguments for the `school-names` subcommand")]
pub(super) struct SchoolNamesArgs {
    #[arg(help = "US jurisdiction code to filter school names (e.g. `OH`)")]
    #[arg(long)]
    pub(super) state: String,

    #[arg(help = "Path to write the sorted, deduplicated school name list")]
    #[arg(long)]
    pub(super) out: PathBuf,
}

pub(super) fn run_school_names(store: &census_store::Store, args: &SchoolNamesArgs) -> Result<()> {
    let schools_path = store.out_dir().join("schools.jsonl");

    let names = collect_school_names(&schools_path, &args.state)?;
    let names_count = names.len();

    write_school_names(&args.out, &names)?;

    println!(
        "wrote {names_count} {} names to {}",
        args.state,
        args.out.display()
    );
    Ok(())
}

fn collect_school_names(schools_path: &Path, wanted: &str) -> Result<BTreeSet<String>> {
    let p = schools_path.display();

    let mut names: BTreeSet<String> = BTreeSet::new();
    let file = File::open(schools_path).with_context(|| format!("opening {p}"))?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.with_context(|| format!("reading line from {p}"))?;
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        let record: Value = serde_json::from_str(&line)
            .with_context(|| format!("parsing school record from {p}"))?;
        let state = record
            .get("state")
            .and_then(|v| v.as_str())
            .map_or("", |value| value);
        if state.to_uppercase() == wanted {
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
    Ok(names)
}

fn write_school_names(out: &Path, names: &BTreeSet<String>) -> Result<()> {
    census_store::read::publish_atomically(out, |temporary| {
        let mut file = File::create(temporary).map_err(|source| census_store::StoreError::Io {
            path: out.to_path_buf(),
            source,
        })?;
        for name in names {
            writeln!(file, "{name}").map_err(|source| census_store::StoreError::Io {
                path: out.to_path_buf(),
                source,
            })?;
        }
        file.flush().map_err(|source| census_store::StoreError::Io {
            path: out.to_path_buf(),
            source,
        })
    })?;
    Ok(())
}
