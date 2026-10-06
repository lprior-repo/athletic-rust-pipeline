#![forbid(unsafe_code)]

#[cfg(test)]
#[macro_use]
#[path = "../../tools/fallible_checks.rs"]
mod fallible_checks;

mod baseline;
mod census;
mod cmd;
mod comments;
mod contract;
mod dump_sheet;
mod helpers;
mod ingress;
mod integrity;
mod json;
mod paths;
mod perf;
mod purity;
mod replay;
mod retry;
mod scaffold;
mod scan;
mod seams;
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
    #[command(
        about = "Run both census-service Criterion targets and record per-benchmark timing and declared throughput; peak RSS is recorded when GNU time is available"
    )]
    Record,
    #[command(
        about = "Re-run both benchmark targets, reject ID or measurement mismatches, and fail regressions beyond the tolerance (default 5%)"
    )]
    Check {
        #[arg(
            help = "Override the default 5% regression tolerance (e.g. `--tolerance 0.1` for 10%)"
        )]
        #[arg(long, default_value_t = 0.05)]
        tolerance: f64,
        #[arg(help = "Reason for running the check; stored alongside the baseline for audit")]
        #[arg(long)]
        reason: Option<String>,
    },
    #[command(
        about = "Run one named group under `perf record --call-graph=dwarf`; prints the exact command and explains why it did not run when `perf` is absent"
    )]
    Profile {
        #[arg(
            help = "Criterion group id to profile, e.g. `census/parse` or `pipeline/result_file`"
        )]
        group: String,
    },
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = "Run the repository quality gate (`tools/gate.sh`)")]
    Gate {
        #[arg(
            help = "Arguments for `tools/gate.sh`, given after `--`: `--update-baseline`, `--allow-increase`"
        )]
        #[arg(last = true, value_name = "GATE_ARG")]
        args: Vec<String>,
    },
    #[command(
        about = "Count forbidden constructs and size-budget overruns in production code; JSON on stdout"
    )]
    Scan,
    #[command(
        about = "Reject comments and prose documentation attributes in project-owned Rust code"
    )]
    Comments,
    #[command(
        about = "Reject panic-producing extraction and its lint overrides in all project Rust"
    )]
    PanicExtraction {
        #[arg(long)]
        root: Option<PathBuf>,
    },
    #[command(
        about = "Assert the architectural constants other work relies on: one line per check, non-zero exit when any check is violated"
    )]
    Contract,
    #[command(
        about = "Check every `crate::…` module reference and every sibling-crate reference in production code against the two allowed-edge tables; JSON on stdout, non-zero exit on a violation"
    )]
    Seams,
    #[command(
        about = "List the type-integrity review candidates of the domain modules; JSON on stdout"
    )]
    Integrity,
    #[command(about = "Rewrite the debt baseline from current measurements")]
    QualityBaseline {
        #[arg(help = "The baseline to write, e.g. `tools/quality-baseline.json`")]
        baseline: PathBuf,
        #[arg(help = "The gate's clippy tallies: `crate<TAB>lint<TAB>count` lines")]
        clippy: PathBuf,
        #[arg(help = "The `scan` report the clippy tallies are ratcheted with")]
        scan: PathBuf,
        #[arg(
            help = "Permit an increase: without it, the update refuses any number that would grow"
        )]
        #[arg(long)]
        allow_increase: bool,
    },
    #[command(
        about = "Compare current measurements against the debt baseline; fail when any metric grew"
    )]
    Ratchet {
        #[arg(help = "The baseline to compare against, e.g. `tools/quality-baseline.json`")]
        baseline: PathBuf,
        #[arg(help = "The gate's clippy tallies: `crate<TAB>lint<TAB>count` lines")]
        clippy: PathBuf,
        #[arg(help = "The `scan` report to compare with the baseline's recorded scan")]
        scan: PathBuf,
    },
    #[command(about = "Prove the `census-domain` dependency tree carries no async or I/O package")]
    DomainPurity,
    #[command(
        about = "Run matching source tests across the workspace with all features; fail when no test matches"
    )]
    #[command(visible_alias = "source-check")]
    SourceTest {
        #[arg(
            help = "Source name as it appears in test names, e.g. `wiaa`, `mshsl`, `wiaa_results`"
        )]
        source: String,
    },
    #[command(
        about = "Run every crate's colocated source tests: the files named `tests.rs` and the inline `#[cfg(test)]` modules, without the `tests/` integration binaries"
    )]
    SourceTests,
    #[command(about = "List the captured fixture files of one source")]
    SourceFixture {
        #[arg(help = "Fixture directory under the crate's `tests/fixtures/`")]
        source: String,
    },
    #[command(
        about = "Replay one source's committed fixture captures through the same parse path its fixture tests use: offline, deterministically, with no network, no store and no clock, so two runs over the same tree print the same bytes"
    )]
    Replay {
        #[arg(
            help = "Fixture directory under the crate's `tests/fixtures/`, e.g. `wiaa`, `mshsl`, `wiaa_results`"
        )]
        name: String,
    },
    #[command(
        about = "Print the core-scope census: the store's own counts from the running deployment (`Census/status`), or a whole core report offline (`census-service report --core`)"
    )]
    CensusStatus {
        #[command(flatten)]
        target: census::Target,
    },
    #[command(
        about = "Print the census over every source: `Report/run` on the running deployment, or `census-service report` offline"
    )]
    Coverage {
        #[command(flatten)]
        target: census::Target,
    },
    #[command(
        about = "Run the pipeline benchmark (`cargo bench -p census-service`), with filters after `--`: `cargo xtask bench -- parser` runs only the parser benchmarks"
    )]
    Bench {
        #[arg(help = "Filter arguments forwarded to `cargo bench`, given after `--`")]
        #[arg(last = true, value_name = "BENCH_ARG")]
        args: Vec<String>,
    },
    #[command(
        about = "Record, check and profile throughput baselines for the `census-service` criterion bench targets"
    )]
    Perf {
        #[command(subcommand)]
        command: PerfCommand,
    },
    #[command(
        about = "Build the census workbook (`.xlsx`) and its text sidecars: `Workbook/run` on the running deployment, or `census-service workbook` offline"
    )]
    Export {
        #[command(flatten)]
        target: census::Target,
        #[arg(
            help = "Where to write the `.xlsx` (defaults to `<store>/out/census-service-<generated-on>.xlsx`)"
        )]
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
        #[arg(help = "Graduation year used for the cohort sheets (2027 = the class of 2027)")]
        #[arg(long, default_value_t = 2027)]
        grad_year: i32,
        #[arg(
            help = "Reduce the best-results sheet over the core scope instead of every approved source"
        )]
        #[arg(long)]
        core: bool,
        #[arg(help = "Cap the per-athlete best-mark sheet at N rows")]
        #[arg(long, value_name = "N")]
        limit: Option<usize>,
    },
    #[command(about = "Scaffold a new source adapter in the directory-module layout")]
    NewSource {
        #[arg(
            help = "Adapter name: lowercase letters, digits and underscores; hyphens become underscores"
        )]
        name: String,
    },
    #[command(about = "Print one `column=value` line per non-empty row for each named sheet")]
    DumpSheet {
        #[arg(help = "Path to the `.xlsx` workbook")]
        workbook: PathBuf,
        #[arg(help = "Sheet names to dump")]
        sheets: Vec<String>,
    },
}

fn main() -> ExitCode {
    if let Err(error) = run() {
        eprintln!("xtask: {error:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Gate { args } => Cmd::new("bash").arg("tools/gate.sh").args(args).run(),
        Command::Scan => scan::run(),
        Command::Comments => comments::run(&paths::repo_root()),
        Command::PanicExtraction { root } => root.map_or_else(
            || comments::extraction::run(&paths::repo_root()),
            |root| comments::extraction::run(&root),
        ),
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
            core,
            limit,
        } => census::export(target, out.as_deref(), grad_year, core, limit),
        Command::NewSource { name } => scaffold::new_source(&name),
        Command::DumpSheet { workbook, sheets } => dump_sheet::run(&workbook, &sheets),
    }
}
