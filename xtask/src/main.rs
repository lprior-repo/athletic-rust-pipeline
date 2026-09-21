//! `xtask`: this repository's developer commands.
//!
//! Every subcommand either runs the real tool — `tools/gate.sh`, `cargo nextest`, the
//! `midwest-census` binary, the scaffold generator — or measures the tree in process (`scan`,
//! `integrity`, `domain-purity`), and prints the exact child command before running it, so a shell
//! session and this harness cannot drift apart.
//!
//! Children run with the repository root as their working directory, whatever directory `xtask`
//! itself was invoked from.

#![forbid(unsafe_code)]

mod baseline;
mod cmd;
mod integrity;
mod json;
mod paths;
mod purity;
mod scaffold;
mod scan;
mod source_fixture;
mod templates;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use cmd::Cmd;
use std::path::{Path, PathBuf};
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
    /// Run the repository quality gate (`tools/gate.sh`).
    Gate {
        /// Arguments for `tools/gate.sh`, given after `--`: `--update-baseline`, `--allow-increase`.
        #[arg(last = true, value_name = "GATE_ARG")]
        args: Vec<String>,
    },
    /// Count forbidden constructs and size-budget overruns in production code; JSON on stdout.
    Scan,
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
    /// Run the midwest-census tests that cover one source (`cargo nextest -E 'test(<source>)'`).
    SourceTest {
        /// Source name as it appears in test names, e.g. `wiaa`, `mshsl`, `wiaa_results`.
        source: String,
    },
    /// List the captured fixture files of one source.
    SourceFixture {
        /// Fixture directory under the crate's `tests/fixtures/`.
        source: String,
    },
    /// Print the core-scope census of a store (`midwest-census report --core`).
    CensusStatus {
        /// Store root (HTTP cache, journals, entity logs, output snapshots).
        #[arg(long, value_name = "DIR")]
        store: PathBuf,
    },
    /// Print the census over every source in a store (`midwest-census report`).
    Coverage {
        /// Store root (HTTP cache, journals, entity logs, output snapshots).
        #[arg(long, value_name = "DIR")]
        store: PathBuf,
    },
    /// Run the pipeline benchmark, once Phase 6 lands `benches/pipeline.rs`.
    Bench,
    /// Scaffold a new source adapter in the directory-module layout.
    NewSource {
        /// Adapter name: lowercase letters, digits and underscores; hyphens become underscores.
        name: String,
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
        Command::SourceTest { source } => source_test(&source),
        Command::SourceFixture { source } => source_fixture::list(&source),
        Command::CensusStatus { store } => census_report(&store, Scope::Core),
        Command::Coverage { store } => census_report(&store, Scope::AllSources),
        Command::Bench => bench(),
        Command::NewSource { name } => scaffold::new_source(&name),
    }
}

/// Which census scope a report covers.
///
/// The shipped CLI spells the core scope `--core` and defaults to every source; there is no
/// `--scope` flag, and `report` writes into `<store>/out/` rather than to a `--out` directory.
#[derive(Clone, Copy, Debug)]
enum Scope {
    /// Core evidence only: Athletic.net and the AthleticLIVE derivative excluded.
    Core,
    /// Every registered source.
    AllSources,
}

/// `cargo nextest run -p midwest-census -E 'test(<source>)'`: one source's tests, nothing else.
fn source_test(source: &str) -> Result<()> {
    Cmd::new("cargo")
        .args(["nextest", "run", "-p", "midwest-census", "-E"])
        .arg(format!("test({source})"))
        .run()
}

/// The census for one store, in the scope the caller asked for.
fn census_report(store: &Path, scope: Scope) -> Result<()> {
    let mut cmd = Cmd::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "midwest-census",
            "--bin",
            "midwest-census",
            "--",
            "--store",
        ])
        .arg(store.display().to_string())
        .arg("report");
    if let Scope::Core = scope {
        cmd = cmd.arg("--core");
    }
    cmd.run()
}

/// Phase 6 owns the benchmark; until `benches/pipeline.rs` exists every number would be invented.
fn bench() -> Result<()> {
    bail!(
        "no benchmark target exists yet: Phase 6 adds `benches/pipeline.rs`, which is also when \
         tools/gate.sh starts running `cargo bench --workspace --no-run`. Until then any latency or \
         throughput figure reported here would be fabricated, so this command refuses to run."
    )
}
