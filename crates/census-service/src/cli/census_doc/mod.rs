//! Render `synthesis/10-measured-census.md` from the pipeline's own snapshots.
//!
//! Reads `<store-out>/report.json`, `<store-out>/athletes.jsonl`,
//! `<store-out>/meets.jsonl`, and the CSVs in `<research>/data/`. Outputs
//! a Markdown census document that is byte-identical to `tools/make_census_doc.py`.
//!
//! This subcommand does not open the Fjall store — it is a pure report
//! generator, so it bypasses the store in [`super::run`].

mod assemble_document;
mod build_coach_rows;
mod build_meets_rows;
mod build_state_rows;
mod compute_metrics;
mod counted_seeds;
mod data_loader;
mod format_sections;
mod ns_key;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;

pub(crate) use self::counted_seeds::Seeds;

/// Arguments for the `census-doc` subcommand.
#[derive(Args, Debug)]
pub(super) struct CensusDocArgs {
    /// Directory containing `report.json`, `athletes.jsonl`, and `meets.jsonl`.
    #[arg(long)]
    pub(super) store_out: PathBuf,

    /// Research root directory containing `data/` and `synthesis/`.
    #[arg(long)]
    pub(super) research: PathBuf,
}

fn md_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push_str("| ");
    out.push_str(&headers.join(" | "));
    out.push_str(" |\n");
    out.push('|');
    for _ in 0..headers.len() {
        out.push_str("---|");
    }
    out.push('\n');
    for row in rows {
        out.push_str("| ");
        out.push_str(&row.join(" | "));
        out.push_str(" |\n");
    }
    out
}

/// Format a number with comma separators.
fn fmt_comma(n: usize) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().rev().collect();
    let mut result = String::new();
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, *ch);
    }
    result
}

/// Format a percentage with one decimal place.
fn fmt_pct(numerator: f64, denominator: f64) -> String {
    if denominator == 0.0 {
        "n/a".to_string()
    } else {
        format!("{:.1} %", 100.0 * numerator / denominator)
    }
}

/// Convert u64 to usize safely; returns MAX on overflow.
fn usize_from_u64(v: u64) -> usize {
    usize::try_from(v).unwrap_or(usize::MAX)
}

/// Build the sports BTreeMap from the report.
fn build_sports_map(report: &serde_json::Value) -> std::collections::BTreeMap<String, usize> {
    report
        .get("class_of_2027_sports")
        .and_then(|v| v.as_object())
        .map(|o| {
            let mut map = std::collections::BTreeMap::new();
            for (k, v) in o {
                if let Some(n) = v.as_u64() {
                    map.insert(k.clone(), usize_from_u64(n));
                }
            }
            map
        })
        .unwrap_or_default()
}

pub(super) fn run_census_doc(args: &CensusDocArgs) -> Result<()> {
    let store_out = &args.store_out;
    let research = &args.research;
    let data_dir = research.join("data");
    let synthesis_dir = research.join("synthesis");

    let (report, seeds, coaches, co2027, recruiting) =
        data_loader::load_all_data(store_out, &data_dir)?;

    let (totals, by_state, schools_by_state, meets) = data_loader::extract_report_data(&report);
    let sports = build_sports_map(&report);

    let state_rows = build_state_rows::build(&by_state, &schools_by_state);
    let coach_rows = build_coach_rows::build(&coaches);
    let metrics = compute_metrics::compute(&co2027, &recruiting);
    let (meets_by_state_rows, provider_rows, provider_count) = build_meets_rows::build(&meets);
    let sports_rows: Vec<Vec<String>> = sports
        .iter()
        .map(|(k, v)| vec![k.clone(), v.to_string()])
        .collect();

    let parts = assemble_document::DocParts {
        report: &report,
        store_out,
        seeds: &seeds,
        state_rows: &state_rows,
        coach_rows: &coach_rows,
        metrics: &metrics,
        recruiting: &recruiting,
        co2027: &co2027,
        meets_by_state_rows: &meets_by_state_rows,
        provider_rows: &provider_rows,
        provider_count,
        sports_rows: &sports_rows,
        totals: &totals,
    };
    let doc = assemble_document::assemble(&parts);

    let out_path = synthesis_dir.join("10-measured-census.md");
    std::fs::write(&out_path, &doc).with_context(|| format!("writing {out_path:?}"))?;

    let summary = make_summary(&report, &out_path, &seeds, &coaches, &co2027, &recruiting)?;
    println!("{summary}");

    Ok(())
}

/// Build the summary JSON string.
fn make_summary(
    report: &serde_json::Value,
    out_path: &Path,
    seeds: &Seeds,
    coaches: &[std::collections::HashMap<String, String>],
    co2027: &[std::collections::HashMap<String, String>],
    recruiting: &[std::collections::HashMap<String, String>],
) -> Result<String> {
    let totals_obj = report.get("totals").cloned().unwrap_or_default();
    let summary = serde_json::json!({
        "wrote": out_path.to_string_lossy().to_string(),
        "totals": totals_obj,
        "seeds": {
            "athletes": seeds.athletes,
            "athletes_multisource": seeds.multisource,
            "distinct_athletic_net_athlete_ids": seeds.distinct_athletic_net_athlete_ids,
            "distinct_athletic_net_meet_ids": seeds.distinct_athletic_net_meet_ids,
        },
        "co2027_rows": co2027.len(),
        "coach_rows": coaches.len(),
        "recruiting_rows": recruiting.len(),
    });
    Ok(serde_json::to_string_pretty(&summary)?)
}
