pub mod check;
pub mod constants;

use anyhow::{bail, Result};
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Debug)]
#[command(about = "What `qa-reports` was asked to verify")]
pub(super) struct QaReportsArgs {
    #[arg(help = "Research workspace root (where `research/midwest/` and `synthesis/` live)")]
    #[arg(long, value_name = "DIR")]
    research: PathBuf,
}

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
    let synthesis_file = research.join("README.md");

    let Some(header_regex) = check::header_re() else {
        bail!("internal: the report header regex failed to compile");
    };
    let mut problems: Vec<String> = Vec::new();
    let mut ok_count: usize = 0;

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

    check::check_evidence(
        &evidence_dir,
        constants::EVIDENCE_START,
        constants::EVIDENCE_END,
        &mut problems,
    );

    check::check_synthesis(&synthesis_file, &mut problems);

    print_and_exit(ok_count, &problems);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::constants::*;

    #[test]
    fn required_sections_compile() {
        for name in REQUIRED_SECTIONS {
            assert!(
                super::check::section_re(name).is_some(),
                "section pattern for {name:?} must compile"
            );
        }
    }

    #[test]
    fn section_re_matches_actual_heading() {
        let text = "## Source";
        assert!(
            super::check::section_re("Source").is_some_and(|rx| rx.is_match(text)),
            "section_re for 'Source' should match '## Source'"
        );
    }
}
