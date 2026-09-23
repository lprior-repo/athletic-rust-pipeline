//! The offline half of the census subcommands: the shipped binary on a store opened in process.
//!
//! `--store <dir>` is the CI and backup-drill path — the only one that works with no server running —
//! and it needs the worker stopped, because the store is single-writer: a running `midwest-serve`
//! holds it and the second handle fails with `FjallError: Locked`.
//!
//! Neither function builds a report: both spawn `midwest-census`, so the numbers an operator reads
//! come from the same binary the deployment runs. [`Cmd`] prints the child command before running it,
//! which is what keeps a shell session and this harness from drifting apart.

use anyhow::Result;
use midwest_census::report::Scope;
use std::path::Path;

use crate::cmd::Cmd;

/// `midwest-census report`, with the store opened in process.
pub(super) fn report(store: &Path, scope: Scope) -> Result<()> {
    let mut cmd = binary(store).arg("report");
    if let Scope::Core = scope {
        cmd = cmd.arg("--core");
    }
    cmd.run()
}

/// `midwest-census workbook` on the same store.
pub(super) fn workbook(
    store: &Path,
    out: Option<&Path>,
    grad_year: i32,
    all_sources: bool,
    limit: Option<usize>,
) -> Result<()> {
    let mut cmd = binary(store).args(["workbook", "--grad-year", &grad_year.to_string()]);
    if let Some(out) = out {
        cmd = cmd.arg("--out").arg(out.display().to_string());
    }
    if all_sources {
        cmd = cmd.arg("--all-sources");
    }
    if let Some(limit) = limit {
        cmd = cmd.args(["--limit", &limit.to_string()]);
    }
    cmd.run()
}

/// The child every offline mode spawns: `cargo run -q -p midwest-census -- --store <dir> <subcommand>`.
///
/// The subcommand and its flags are appended by the caller, so the store argument cannot drift out of
/// the position the binary expects.
fn binary(store: &Path) -> Cmd {
    Cmd::new("cargo")
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
}
