use census_report::workbook::reference_comparison::{compare, ComparisonRequest};
use clap::Args;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub(super) struct CompareReferenceArgs {
    #[arg(
        long,
        help = "Manifested immutable publication workbook; no store is opened"
    )]
    workbook: PathBuf,
    #[arg(long, help = "External comparison-only workbook; never imported")]
    reference: PathBuf,
    #[arg(long, value_parser = ["athletic-2026-us-boys-grade11-v1"])]
    reference_profile: String,
    #[arg(
        long,
        help = "High-school graduation year, not grade or NCAA eligibility certification"
    )]
    graduation_year: i16,
}

pub(super) fn run(args: &CompareReferenceArgs) -> ! {
    let request = ComparisonRequest {
        workbook: &args.workbook,
        reference: &args.reference,
        reference_profile: &args.reference_profile,
        graduation_year: args.graduation_year,
    };
    let exit = match compare(&request) {
        Ok(summary) => {
            let exit = exit_code(Ok(summary.passed()));
            let output = std::io::stdout();
            let mut writer = output.lock();
            match serde_json::to_writer(&mut writer, &summary)
                .map_err(std::io::Error::other)
                .and_then(|()| writer.write_all(b"\n"))
                .and_then(|()| writer.flush())
            {
                Ok(()) => exit,
                Err(_) => {
                    eprintln!("compare-reference: aggregate output failed");
                    exit_code(Err(()))
                }
            }
        }
        Err(_) => {
            eprintln!("compare-reference: input, integrity or resource validation failed; no complete comparison");
            exit_code(Err(()))
        }
    };
    std::process::exit(exit)
}

fn exit_code(required_checks: Result<bool, ()>) -> i32 {
    match required_checks {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(()) => 2,
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    #[test]
    fn required_check_and_error_exit_codes_remain_distinct() {
        assert_eq!(super::exit_code(Ok(true)), 0);
        assert_eq!(super::exit_code(Ok(false)), 1);
        assert_eq!(super::exit_code(Err(())), 2);
    }

    #[test]
    fn comparator_requires_explicit_reference_profile_and_graduation_year() {
        let base = [
            "census-service",
            "compare-reference",
            "--workbook",
            "generation/workbook.xlsx",
            "--reference",
            "comparison.xlsx",
        ];
        assert!(crate::cli::Cli::try_parse_from(base).is_err());
        let arguments = base.into_iter().chain([
            "--reference-profile",
            "athletic-2026-us-boys-grade11-v1",
            "--graduation-year",
            "2027",
        ]);
        assert!(crate::cli::Cli::try_parse_from(arguments).is_ok());
    }
}
