//! `xtask`: this repository's developer commands.
//!
//! Every subcommand either runs the real tool — `tools/gate.sh`, `cargo nextest`, the
//! `census-service` binary, the scaffold generator — or measures the tree in process (`scan`,
//! `integrity`, `domain-purity`), and prints the exact child command before running it, so a shell
//! session and this harness cannot drift apart.
//!
//! Children run with the repository root as their working directory, whatever directory `xtask`
//! itself was invoked from. The census subcommands ([`census`]) can submit the running deployment's
//! own handlers instead of running the binary, which is the only mode that works while
//! `census-serve` holds the store.

#![forbid(unsafe_code)]

mod baseline;
mod census;
mod cmd;
mod contract;
mod dump_sheet;
mod helpers;
mod ingress;
mod integrity;
mod json;
mod kani;
mod paths;
mod perf;
mod purity;
mod replay;
mod retry;
mod scaffold;
mod seams;
mod scan;
mod source_fixture;
mod templates;

use anyhow::Result;
use clap::{Parser, Subcommand};
use cmd::Cmd;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "xtask",
    about = "Developer commands for athletic-rust-pipeline: quality gate, source tests and fixtures, census reports, adapter scaffolds",
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand, Debug)]
enum PerfCommand {
    /// Run the `census-service` criterion bench targets and write a baseline recording wall time,
    /// throughput and peak RSS per group, stamped with hardware and toolchain metadata.
    Record,
    /// Re-run the bench targets, compare throughput against the recorded baseline, and fail when
    /// any group regresses past the tolerance (default 5%, overridable with `--tolerance`).
    Check {
        /// Override the default 5% regression tolerance (e.g. `--tolerance 0.1` for 10%).
        #[arg(long, default_value_t = 0.05)]
        tolerance: f64,
        /// Reason for running the check; stored alongside the baseline for audit.
        #[arg(long)]
        reason: Option<String>,
    },
    /// Run one named group under `perf record --call-graph=dwarf`; prints the exact command
    /// and explains why it did not run when `perf` is absent.
    Profile {
        /// Criterion group id to profile, e.g. `census/parse` or `pipeline/result_file`.
        group: String,
    },
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the repository quality gate (`tools/gate.sh`).
    Gate {
        /// Arguments for `tools/gate.sh`, given after `--`: `--update-baseline`, `--allow-increase`.
        #[arg(last = true, value_name = "GATE_ARG")]
        args: Vec<String>,
    },
    /// Count forbidden constructs and size-budget overruns in production code; JSON on stdout.
    Scan,
    /// Assert the architectural constants other work relies on: one line per check, non-zero exit
    /// when any of the eight is violated.
    Contract,
    /// Check every `crate::…` module reference and every sibling-crate reference in production
    /// code against the two allowed-edge tables; JSON on stdout, non-zero exit on a violation.
    Seams,
    /// List the type-integrity review candidates of the domain modules; JSON on stdout.
    Integrity,
    /// Rewrite the debt baseline from current measurements.
    QualityBaseline {
        /// The baseline to write, e.g. `tools/quality-baseline.json`.
        baseline: PathBuf,
        /// The gate's clippy tallies: `crate<TAB>lint<TAB>count` lines.
        clippy: PathBuf,
        /// The `scan` report the clippy tallies are ratcheted with.
        scan: PathBuf,
        /// Permit an increase: without it, the update refuses any number that would grow.
        #[arg(long)]
        allow_increase: bool,
    },
    /// Compare current measurements against the debt baseline; fail when any metric grew.
    Ratchet {
        /// The baseline to compare against, e.g. `tools/quality-baseline.json`.
        baseline: PathBuf,
        /// The gate's clippy tallies: `crate<TAB>lint<TAB>count` lines.
        clippy: PathBuf,
        /// The `scan` report to compare with the baseline's recorded scan.
        scan: PathBuf,
    },
    /// Prove the `census-domain` dependency tree carries no async or I/O package.
    DomainPurity,
    /// Run the census-service tests that cover one source (`cargo nextest -E 'test(<source>)'`).
    #[command(visible_alias = "source-check")]
    SourceTest {
        /// Source name as it appears in test names, e.g. `wiaa`, `mshsl`, `wiaa_results`.
        source: String,
    },
    /// Run every crate's colocated source tests: the files named `tests.rs` and the inline
    /// `#[cfg(test)]` modules, without the `tests/` integration binaries.
    SourceTests,
    /// List the captured fixture files of one source.
    SourceFixture {
        /// Fixture directory under the crate's `tests/fixtures/`.
        source: String,
    },
    /// Replay one source's committed fixture captures through the same parse path its fixture tests
    /// use: offline, deterministically, with no network, no store and no clock, so two runs over the
    /// same tree print the same bytes.
    Replay {
        /// Fixture directory under the crate's `tests/fixtures/`, e.g. `wiaa`, `mshsl`, `wiaa_results`.
        name: String,
    },
    /// Print the core-scope census: the store's own counts from the running deployment
    /// (`Census/status`), or a whole core report offline (`census-service report --core`).
    CensusStatus {
        #[command(flatten)]
        target: census::Target,
    },
    /// Print the census over every source: `Report/run` on the running deployment, or
    /// `census-service report` offline.
    Coverage {
        #[command(flatten)]
        target: census::Target,
    },
    /// Run the pipeline benchmark (`cargo bench -p census-service`), with filters after `--`:
    /// `cargo xtask bench -- parser` runs only the parser benchmarks.
    Bench {
        /// Filter arguments forwarded to `cargo bench`, given after `--`.
        #[arg(last = true, value_name = "BENCH_ARG")]
        args: Vec<String>,
    },
    /// Record, check and profile throughput baselines for the `census-service` criterion bench targets.
    Perf {
        #[command(subcommand)]
        command: PerfCommand,
    },
    /// Build the census workbook (`.xlsx`) and its text sidecars: `Workbook/run` on the running
    /// deployment, or `census-service workbook` offline.
    Export {
        #[command(flatten)]
        target: census::Target,
        /// Where to write the `.xlsx` (defaults to `<store>/out/census-service-<generated-on>.xlsx`).
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
        /// Graduation year used for the cohort sheets (2027 = the class of 2027).
        #[arg(long, default_value_t = 2027)]
        grad_year: i32,
        /// Reduce the best-results sheet over every source rather than the core scope alone.
        #[arg(long)]
        all_sources: bool,
        /// Cap the per-athlete best-mark sheet at N rows.
        #[arg(long, value_name = "N")]
        limit: Option<usize>,
    },
    /// Scaffold a new source adapter in the directory-module layout.
    NewSource {
        /// Adapter name: lowercase letters, digits and underscores; hyphens become underscores.
        name: String,
    },
    /// Print one `column=value` line per non-empty row for each named sheet.
    DumpSheet {
        /// Path to the `.xlsx` workbook.
        workbook: PathBuf,
        /// Sheet names to dump.
        sheets: Vec<String>,
    },
    /// Verify `census-domain` invariants via `cargo kani`: fixed-point bounds, PR comparison
    /// laws, identity contradiction, redirect cycle, retry limits, terminal state,
    /// StoreBatch arithmetic, and `CENSUS_SCOPE` scope size.
    Kani {
        /// Harness names to verify; when omitted, all mandatory harnesses are run.
        #[arg(last = true, value_name = "HARNESS")]
        harnesses: Vec<String>,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Gate { args } => Cmd::new("bash").arg("tools/gate.sh").args(args).run(),
        Command::Scan => scan::run(),
        Command::Contract => contract::run(),
        Command::Seams => seams::run(),
        Command::Integrity => integrity::run(),
        Command::QualityBaseline {
            baseline,
            clippy,
            scan,
            allow_increase,
        } => baseline::update(&baseline, &clippy, &scan, allow_increase),
        Command::Ratchet {
            baseline,
            clippy,
            scan,
        } => baseline::ratchet(&baseline, &clippy, &scan),
        Command::DomainPurity => purity::run(),
        Command::SourceTest { source } => helpers::source_test(&source),
        Command::SourceTests => helpers::source_tests(),
        Command::SourceFixture { source } => source_fixture::list(&source),
        Command::Replay { name } => replay::run(&name),
        Command::CensusStatus { target } => census::status(target),
        Command::Coverage { target } => census::coverage(target),
        Command::Bench { args } => helpers::bench(&args),
        Command::Perf { command } => match command {
            PerfCommand::Record => perf::run_record(),
            PerfCommand::Check { tolerance, reason } => perf::run_check(tolerance, reason),
            PerfCommand::Profile { group } => perf::run_profile(&group),
        },
        Command::Export {
            target,
            out,
            grad_year,
            all_sources,
            limit,
        } => census::export(target, out.as_deref(), grad_year, all_sources, limit),
        Command::NewSource { name } => scaffold::new_source(&name),
        Command::DumpSheet { workbook, sheets } => dump_sheet::run(&workbook, &sheets),
        Command::Kani { harnesses } => kani::run(&harnesses),
    }
}
