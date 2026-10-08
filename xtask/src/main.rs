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
#[path = "main/quality.rs"]
mod quality;
mod replay;
mod retry;
mod scaffold;
mod scan;
mod seams;
mod source_fixture;
mod storage_ab;
mod templates;

use anyhow::Result;
use clap::{Parser, Subcommand};
use cmd::Cmd;
use perf::PerfCommand;
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
    #[command(flatten)]
    Quality(quality::QualityCommand),
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
    #[command(flatten)]
    Census(census::CensusCommand),
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
    #[command(
        about = "Run the census-store keyspace A/B benchmark (single vs split evidence/derived keyspaces)"
    )]
    StorageAb {
        #[arg(help = "Arguments forwarded to the example, given after --")]
        #[arg(last = true, value_name = "AB_ARG")]
        args: Vec<String>,
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
        Command::PanicExtraction { root } => panic_extraction(root),
        Command::Contract => contract::run(),
        Command::Seams => seams::run(),
        Command::Integrity => integrity::run(),
        Command::Quality(command) => command.run(),
        Command::DomainPurity => purity::run(),
        Command::SourceTest { source } => helpers::source_test(&source),
        Command::SourceTests => helpers::source_tests(),
        Command::SourceFixture { source } => source_fixture::list(&source),
        Command::Replay { name } => replay::run(&name),
        Command::Census(command) => command.run(),
        Command::Bench { args } => helpers::bench(&args),
        Command::Perf { command } => run_perf(command),
        Command::NewSource { name } => scaffold::new_source(&name),
        Command::DumpSheet { workbook, sheets } => dump_sheet::run(&workbook, &sheets),
        Command::StorageAb { args } => storage_ab::run(args),
    }
}

fn panic_extraction(root: Option<PathBuf>) -> Result<()> {
    root.map_or_else(
        || comments::extraction::run(&paths::repo_root()),
        |root| comments::extraction::run(&root),
    )
}

fn run_perf(command: PerfCommand) -> Result<()> {
    match command {
        PerfCommand::Record => perf::run_record(),
        PerfCommand::Check { tolerance, reason } => perf::run_check(tolerance, reason),
        PerfCommand::Profile { group } => perf::run_profile(&group),
    }
}
