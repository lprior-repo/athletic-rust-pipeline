//! One audit run: read the three inputs, index them, scan the raw bodies, cross-check the records,
//! and print every section in the order the retained outputs fixed.

use std::collections::BTreeMap;
use std::fs;

use indexmap::IndexMap;
use serde_json::Value;

use crate::evidence;
use crate::inputs;
use crate::outcomes;
use crate::pages;
use crate::perpage;
use crate::report;
use crate::rows::Regexes;
use crate::samples;
use crate::verdicts;
use crate::Args;

/// The parsed records, keyed by the digest the evidence records join on.
fn read_parsed(
    parsed_files: &[(String, String)],
) -> Result<BTreeMap<String, Value>, Box<dyn std::error::Error>> {
    parsed_files
        .iter()
        .map(|(name, path)| -> Result<_, Box<dyn std::error::Error>> {
            let blob = fs::read(path)?;
            let rec: Value = serde_json::from_slice(&blob)?;
            Ok::<_, Box<dyn std::error::Error>>((name.clone(), rec))
        })
        .collect::<Result<_, _>>()
}

/// Run the inventory over the directories `args` names.
pub(crate) fn run(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let raw_files = inputs::read_dir(&args.raw);
    let evidence_files = inputs::read_dir(&args.evidence);
    let parsed_files = inputs::read_dir(&args.parsed);
    report::print_header(
        &args.label,
        raw_files.len(),
        evidence_files.len(),
        parsed_files.len(),
    );

    let regexes = Regexes::compile()?;

    let evidence = evidence::index(&evidence_files)?;
    evidence.print_summary();
    let parsed_records = read_parsed(&parsed_files)?;

    let mut verdict_totals: IndexMap<String, usize> = IndexMap::new();
    let scan = pages::scan(
        &raw_files,
        &evidence.join,
        &parsed_records,
        args.samples,
        &mut verdict_totals,
        &regexes,
    )?;
    let crosscheck = outcomes::crosscheck(&evidence.records, &parsed_records);

    report::print_link_inventory(&scan.link_totals);
    report::print_count_vs_rows(&scan.count_vs_rows);
    report::print_verdict_header();
    verdicts::print_recount(&scan.per_page);
    verdicts::print_parser(&scan.per_page);
    verdicts::populate_totals(&scan.per_page, &mut verdict_totals);
    verdicts::print_issue_classes(&scan.per_page);
    verdicts::print_co_mingled(&scan.per_page);
    verdicts::print_pagination(&scan.per_page);
    outcomes::print_crosscheck(&crosscheck);
    outcomes::print_failure_classes(&crosscheck);
    outcomes::print_unexplained(&crosscheck);
    report::print_verdict_totals(&verdict_totals);
    report::print_page_filter(&scan.filter_totals);
    report::print_interstitials(&scan.interstitials);
    report::print_mismatches(&scan.digest_mismatch, &scan.byte_mismatch);
    samples::print(&scan.class_samples);
    perpage::print(&scan.per_page);
    Ok(())
}
