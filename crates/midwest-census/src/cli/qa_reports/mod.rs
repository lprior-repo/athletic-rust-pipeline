//! The `qa-reports` subcommand: verify every assignment report exists, carries the required schema
//! headings, starts with the correct numbered header, has a `Status:` line near the top, and that
//! each gap-phase follow-up (31–46) has a non-empty evidence directory.
//!
//! The research workspace root is supplied via `--research`; from there the tool looks for
//! `research/midwest/*.md`, `research/midwest/evidence/gaps/<N>/`, and
//! `synthesis/09-gap-phase-consolidation-2026-09-20.md`.
//!
//! Exit code 0 = all present and schema-complete; 1 = problems listed. Read-only.

pub mod check;
pub mod constants;

use anyhow::{bail, Result};
use clap::Args;
use std::path::PathBuf;

/// What `qa-reports` was asked to verify.
#[derive(Args, Debug)]
pub(super) struct QaReportsArgs {
    /// Research workspace root (where `research/midwest/` and `synthesis/` live).
    #[arg(long, value_name = "DIR")]
    research: PathBuf,
}

/// Run the qa-reports subcommand.
/// Report results and exit if problems exist.
fn print_and_exit(ok_count: usize, problems: &[String]) {
    println!(
        "\nreports present: {ok_count}/{}",
        constants::EXPECTED.len()
    );
    if !problems.is_empty() {
        println!("\nPROBLEMS:");
        for p in problems {
            println!(" - {p}");
        }
        std::process::exit(1);
    }
    println!("all reports present and schema-complete");
}
pub(super) fn run_qa_reports(args: &QaReportsArgs) -> Result<()> {
    let research = &args.research;
    let reports_dir = research.join("research").join("midwest");
    let evidence_dir = reports_dir.join("evidence").join("gaps");
    let synthesis_file = research
        .join("synthesis")
        .join("09-gap-phase-consolidation-2026-09-20.md");

    let Some(header_regex) = check::header_re() else {
        bail!("internal: the report header regex failed to compile");
    };
    let mut problems: Vec<String> = Vec::new();
    let mut ok_count: usize = 0;

    // Pre-build section regexes once.
    let section_rees: Vec<(&str, Option<regex::Regex>)> = constants::REQUIRED_SECTIONS
        .iter()
        .map(|name| (*name, check::section_re(name)))
        .collect();

    for (num, fname) in constants::EXPECTED {
        let path = reports_dir.join(fname);
        if check::check_report(
            num,
            fname,
            &path,
            &section_rees,
            &header_regex,
            &mut problems,
        ) {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| format!("error reading {}: {e}", path.display()));
            ok_count = ok_count.saturating_add(1);
            println!("OK {:02} {:<55} {:>7} chars", num, fname, text.len());
        }
    }

    // Check evidence directories for gap-phase reports (31–46).
    check::check_evidence(
        &evidence_dir,
        constants::EVIDENCE_START,
        constants::EVIDENCE_END,
        &mut problems,
    );

    // Check synthesis file.
    check::check_synthesis(&synthesis_file, &mut problems);

    print_and_exit(ok_count, &problems);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::constants::*;

    /// Verify the 46 entries cover 1–46 with no gaps or duplicates.
    #[test]
    fn expected_numbers_1_to_46() {
        let mut prev = 0usize;
        for (num, _fname) in EXPECTED {
            assert_eq!(num, prev + 1, "number {num} is not sequential after {prev}");
            prev = num;
        }
        assert_eq!(prev, 46, "last entry is {prev}, expected 46");
    }

    /// Verify every filename matches the expected NNN-slug.md pattern.
    #[test]
    fn expected_filenames_valid() {
        for (num, fname) in EXPECTED {
            let expected = format!("{num:02}-");
            assert!(
                fname.starts_with(&expected),
                "{fname} does not start with {expected}"
            );
            assert!(fname.ends_with(".md"), "{fname} is not a .md file");
        }
    }

    /// Verify each required section compiles into a valid regex.
    #[test]
    fn required_sections_compile() {
        for name in REQUIRED_SECTIONS {
            assert!(
                super::check::section_re(name).is_some(),
                "section pattern for {name:?} must compile"
            );
        }
    }

    /// Verify section_re matches an actual heading from a report.
    #[test]
    fn section_re_matches_actual_heading() {
        let text = "## Source";
        assert!(
            super::check::section_re("Source").is_some_and(|rx| rx.is_match(text)),
            "section_re for 'Source' should match '## Source'"
        );
    }
}
