//! The merge report: per-state accounting, every rejection with its evidence, and the fragments
//! that could not be read at all.

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

use super::{KeptRows, Rejection, StateCounts};

pub(super) fn print_report(
    kept: &KeptRows,
    per_state: &BTreeMap<String, StateCounts>,
    rejects: &[Rejection],
    broken: &[(String, String)],
    out: &Path,
    report: &Path,
) {
    let total_kept: usize = per_state.values().map(|c| c.kept).sum();
    let states: usize = per_state.len();
    let with_email: usize = kept
        .values()
        .filter(|r| !r.public_professional_email.trim().is_empty() || !r.ad_email.trim().is_empty())
        .count();
    let with_coach_email: usize = kept
        .values()
        .filter(|r| !r.public_professional_email.trim().is_empty())
        .count();
    let ad_rows: usize = kept
        .values()
        .filter(|r| r.role.trim().to_lowercase().contains("director"))
        .count();
    println!(
        "kept {} rows from {} states -> {}",
        total_kept,
        states,
        out.display()
    );
    println!(
        "  with any email: {}  coach email: {}  AD rows: {}",
        with_email, with_coach_email, ad_rows
    );
    println!(
        "  rejected: {}  unusable fragments: {}",
        rejects.len(),
        broken.len()
    );
    println!("  report: {}", report.display());
    if let Err(error) = write_report(kept, per_state, rejects, broken, out, report) {
        eprintln!("could not write {}: {error}", report.display());
    }
}

/// Write the markdown report: per-state accounting, every rejection with its evidence, and the
/// fragments that could not be read at all.
pub(super) fn write_report(
    kept: &KeptRows,
    per_state: &BTreeMap<String, StateCounts>,
    rejects: &[Rejection],
    broken: &[(String, String)],
    out: &Path,
    report: &Path,
) -> Result<()> {
    let mut doc = String::new();
    doc.push_str("# merge-coaches report\n\n");
    doc.push_str(&format!("Output: `{}`\n\n", out.display()));
    doc.push_str("| state | rows | kept | duplicates | rejected |\n|---|---|---|---|---|\n");
    for (state, counts) in per_state {
        doc.push_str(&format!(
            "| {state} | {} | {} | {} | {} |\n",
            counts.rows, counts.kept, counts.duplicates, counts.rejected
        ));
    }
    let total: usize = kept.len();
    doc.push_str(&format!("\nTotal kept: {total} unique rows\n"));
    if !broken.is_empty() {
        doc.push_str("\n## Unusable fragments\n\n");
        for (file, problem) in broken {
            doc.push_str(&format!("- `{file}`: {problem}\n"));
        }
    }
    if !rejects.is_empty() {
        doc.push_str(
            "\n## Rejections\n\n| state | line | school | coach | role | reason | source |\n",
        );
        doc.push_str("|---|---|---|---|---|---|---|\n");
        for row in rejects {
            doc.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                row.state,
                row.line_no,
                row.school,
                row.coach_name,
                row.role,
                row.reason,
                row.source_url
            ));
        }
    }
    std::fs::write(report, doc).with_context(|| format!("writing {}", report.display()))?;
    Ok(())
}
