use super::{baseline, candidate};
use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(crate) enum PerfCommand {
    #[command(
        about = "Record Criterion timing, sample-tail timing, throughput and required GNU time RSS/Valgrind DHAT heap allocation measurements"
    )]
    Record {
        #[arg(
            long,
            help = "Write a new candidate artifact; refuses existing outputs and the canonical baseline"
        )]
        output: Option<PathBuf>,
    },
    #[command(
        about = "Re-run all three benchmark targets, reject ID or measurement mismatches, and fail regressions beyond the tolerance (default 5%; the sample-maximum tail bound is floored at 20%, the GNU time peak-RSS bound at a seventh of its recorded baseline (1 MiB minimum) and the wall-time/throughput bounds at the case's recorded tail-to-mean spread, capped at the same 20%)"
    )]
    Check {
        #[arg(
            help = "Override the default 5% regression tolerance (e.g. `--tolerance 0.1` for 10%; the sample-maximum tail bound is floored at 20%, the GNU time peak-RSS bound at a seventh of its recorded baseline (1 MiB minimum) and the wall-time/throughput bounds at the case's recorded tail-to-mean spread, capped at the same 20%)"
        )]
        #[arg(long, default_value_t = 0.05)]
        tolerance: f64,
        #[arg(
            long,
            help = "Comparison reason; does not waive environment or metric mismatches"
        )]
        reason: Option<String>,
        #[arg(
            long,
            help = "Read this independently reviewed baseline instead of the canonical baseline"
        )]
        baseline: Option<PathBuf>,
    },
    #[command(
        about = "Compare retained baseline and candidate artifacts without measuring or promoting a baseline"
    )]
    Compare {
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long)]
        candidate: PathBuf,
        #[arg(long, default_value_t = 0.05)]
        tolerance: f64,
        #[arg(
            long,
            help = "Comparison reason; does not waive environment or metric mismatches"
        )]
        reason: Option<String>,
    },
    #[command(about = "Profile a named group with perf record --call-graph=dwarf")]
    Profile {
        #[arg(help = "Criterion group ID, for example census/parse or pipeline/result_file")]
        group: String,
    },
}

impl PerfCommand {
    pub(crate) fn run(self) -> Result<()> {
        match self {
            Self::Record { output: None } => super::run_record(),
            Self::Record {
                output: Some(output),
            } => candidate::record(&output, super::measure),
            Self::Check {
                tolerance,
                reason,
                baseline: None,
            } => super::run_check(tolerance, reason),
            Self::Check {
                tolerance,
                reason,
                baseline: Some(path),
            } => super::check(baseline::load_baseline_from(&path)?, tolerance, reason),
            Self::Compare {
                baseline,
                candidate: path,
                tolerance,
                reason,
            } => candidate::compare_files(&baseline, &path, tolerance, reason.as_deref()),
            Self::Profile { group } => super::run_profile(&group),
        }
    }
}
