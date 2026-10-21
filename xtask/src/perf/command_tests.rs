use crate::perf::PerfCommand;
use clap::Parser;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Parser)]
struct Arguments {
    #[command(subcommand)]
    command: PerfCommand,
}

#[test]
fn legacy_record_and_check_defaults_remain_available() -> TestResult {
    let record = Arguments::try_parse_from(["perf", "record"])?;
    check!(matches!(
        record.command,
        PerfCommand::Record { output: None }
    ));
    let check = Arguments::try_parse_from(["perf", "check"])?;
    let PerfCommand::Check {
        tolerance,
        reason,
        baseline,
    } = check.command
    else {
        return Err("check command was not retained".into());
    };
    check!(eq; tolerance, 0.05);
    check!(reason.is_none());
    check!(baseline.is_none());
    Ok(())
}

#[test]
fn candidate_output_and_explicit_live_baseline_are_separate_paths() -> TestResult {
    let record = Arguments::try_parse_from(["perf", "record", "--output", "candidate.json"])?;
    let PerfCommand::Record { output } = record.command else {
        return Err("candidate record was not parsed".into());
    };
    check!(eq; output, Some(PathBuf::from("candidate.json")));
    let check = Arguments::try_parse_from([
        "perf",
        "check",
        "--baseline",
        "retained.json",
        "--reason",
        "independent control",
    ])?;
    let PerfCommand::Check {
        baseline,
        tolerance,
        reason,
    } = check.command
    else {
        return Err("explicit baseline check was not parsed".into());
    };
    check!(eq; baseline, Some(PathBuf::from("retained.json")));
    check!(eq; tolerance, 0.05);
    check!(eq; reason.as_deref(), Some("independent control"));
    Ok(())
}

#[test]
fn artifact_comparison_requires_both_explicit_inputs() -> TestResult {
    for args in [
        vec!["perf", "compare"],
        vec!["perf", "compare", "--baseline", "retained.json"],
        vec!["perf", "compare", "--candidate", "candidate.json"],
    ] {
        check!(Arguments::try_parse_from(args).is_err());
    }
    let parsed = Arguments::try_parse_from([
        "perf",
        "compare",
        "--baseline",
        "retained.json",
        "--candidate",
        "candidate.json",
    ])?;
    let PerfCommand::Compare {
        baseline,
        candidate,
        tolerance,
        reason,
    } = parsed.command
    else {
        return Err("artifact comparison was not parsed".into());
    };
    check!(eq; baseline, PathBuf::from("retained.json"));
    check!(eq; candidate, PathBuf::from("candidate.json"));
    check!(eq; tolerance, 0.05);
    check!(reason.is_none());
    Ok(())
}
