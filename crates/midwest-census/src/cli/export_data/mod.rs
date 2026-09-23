//! Export the canonical Midwest census snapshots into CSV data products.
//!
//! Input : `<store-out>/{athletes,schools,coaches,meets}.jsonl` (consolidated snapshots)
//! Output: `<data>/canonical-*.csv`, `athleticnet-athlete-seeds.csv`, `recruiting-co2027.csv`
//!
//! Usage: `midwest-census <global-args> export-data --store-out <DIR> --data <DIR>`
//!
//! Everything here is a projection of the consolidated store; nothing is re-derived or guessed.
//! Athletic.net ids come only from `legacy_athletic_net` identities that a
//! non-Athletic.net source published.

pub mod athletes;
pub mod coaches;
pub mod csv;
pub mod helpers;
pub mod meets;
pub mod recruiting;
pub mod schools;

use anyhow::{Context, Result};
use clap::Args;
use std::fs;
use std::path::PathBuf;

use midwest_census::store::read::read_rows;

/// Arguments for the `export-data` subcommand.
#[derive(Debug, Args)]
pub(super) struct ExportDataArgs {
    /// Directory containing the consolidated `*.jsonl` snapshots (athletes, schools, coaches, meets).
    #[arg(long)]
    pub(super) store_out: PathBuf,

    /// Directory where the CSV data products are written.
    #[arg(long)]
    pub(super) data: PathBuf,
}

/// The four JSONL tables the export projects from.
struct ExportTables<'a> {
    schools: &'a [serde_json::Value],
    athletes: &'a [serde_json::Value],
    coaches: &'a [serde_json::Value],
    meets: &'a [serde_json::Value],
}

/// What the export counted, as reported back to the caller.
struct ExportCounts {
    co27: usize,
    multi: usize,
    with_coach: usize,
    with_email: usize,
}

/// Run the export-data subcommand.
/// Print the final summary statistics.
fn print_summary(
    store_out: &std::path::Path,
    data: &std::path::Path,
    tables: &ExportTables<'_>,
    counts: &ExportCounts,
) {
    let files: Vec<String> = if let Ok(entries) = fs::read_dir(data) {
        entries
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect()
    } else {
        Vec::new()
    };
    let mut files = files;
    files.sort();
    println!(
        "{}",
        serde_json::json!({
            "out_dir": store_out.to_string_lossy(),
            "data_dir": data.to_string_lossy(),
            "schools": tables.schools.len(),
            "athletes": tables.athletes.len(),
            "co2027": counts.co27,
            "athletes_multi_source": counts.multi,
            "coaches": tables.coaches.len(),
            "meets": tables.meets.len(),
            "co2027_with_school_coach": counts.with_coach,
            "co2027_with_any_coach_email": counts.with_email,
            "files": files,
        })
    );
}
pub(super) fn run_export_data(args: &ExportDataArgs) -> Result<()> {
    let store_out = &args.store_out;
    let data = &args.data;

    fs::create_dir_all(data).with_context(|| format!("creating data dir {}", data.display()))?;

    let schools = read_rows::<serde_json::Value>(&store_out.join("schools.jsonl"))?;
    let athletes = read_rows::<serde_json::Value>(&store_out.join("athletes.jsonl"))?;
    let coaches = read_rows::<serde_json::Value>(&store_out.join("coaches.jsonl"))?;
    let meets = read_rows::<serde_json::Value>(&store_out.join("meets.jsonl"))?;
    let by_school: std::collections::HashMap<&str, &serde_json::Value> = schools
        .iter()
        .filter_map(|s| s.get("id").and_then(|id| id.as_str()).map(|id| (id, s)))
        .collect();

    // -- canonical schools -------------------------------------------------------------------
    schools::write_canonical_schools(&schools, data)?;

    // -- canonical coaches -------------------------------------------------------------------
    coaches::write_canonical_coaches(&coaches, &by_school, data)?;

    // -- canonical meets ---------------------------------------------------------------------
    meets::write_canonical_meets(&meets, data)?;

    // -- athletes ----------------------------------------------------------------------------
    let (co27, multi) = athletes::write_athletes(&athletes, &by_school, data)?;

    // -- recruiting projection ---------------------------------------------------------------
    let coach_index = recruiting::build_coach_index(&coaches);
    let (with_coach, with_email) =
        recruiting::write_recruiting(&athletes, &coach_index, &by_school, data)?;

    print_summary(
        store_out,
        data,
        &ExportTables {
            schools: &schools,
            athletes: &athletes,
            coaches: &coaches,
            meets: &meets,
        },
        &ExportCounts {
            co27,
            multi,
            with_coach,
            with_email,
        },
    );

    Ok(())
}

#[cfg(test)]
mod tests;
