//! The census subcommands: `census-status`, `coverage` and `export`.
//!
//! Each has two modes:
//!
//! * `--store <dir>` runs the shipped `midwest-census` binary, which opens the store in process.
//!   That is the backup-drill and CI path — it is the only one that works with no server running —
//!   and it needs the worker stopped, because the store is single-writer: a running `midwest-serve`
//!   holds it, and the second handle fails with `FjallError: Locked`.
//! * no flag, or `--ingress <origin>`, submits the same work to the running deployment
//!   (`Census/status`, `Report/run`, `Workbook/run`) and never opens the store. That is the
//!   mode that works *while* the census serves, and it is the default with the project node origin.
//!
//! Both modes announce what they ran before running it — the offline mode through
//! [`crate::cmd::Cmd`]'s
//! `+ <command>` line, the ingress mode through a `+ POST <origin>restate/call/<Service>/<handler>`
//! line — and both print the same operator-facing lines from what came back.
//!
//! The two halves live beside this façade ([`offline`] and [`service`]) because they are two
//! transports for one question, not two features: the mode split is a `match` in each entry point
//! below, so an entry point's whole shape is visible in one screen.

use anyhow::{bail, Result};
use census_report::report::Scope;
use std::path::{Path, PathBuf};

use crate::ingress;

mod offline;
mod service;

#[cfg(test)]
#[path = "census/tests.rs"]
mod tests;

/// Where a census subcommand reads from: the store directly, or the running deployment.
///
/// `--store` selects the offline mode; without it the command submits to the running deployment, so
/// the flag-free form works exactly while `midwest-serve` holds the store. The two are exclusive by
/// construction: `--store` opens the store in process, which is only possible with the worker
/// stopped, and the ingress asks the worker that holds it.
#[derive(clap::Args, Debug)]
#[group(multiple = false)]
pub struct Target {
    /// Store root (HTTP cache, journals, entity logs, output snapshots). Needs `midwest-serve`
    /// stopped: the store is single-writer.
    #[arg(long, value_name = "DIR")]
    store: Option<PathBuf>,
    /// Restate ingress origin of the running deployment; `--ingress` alone uses the project node
    /// origin (http://127.0.0.1:18095/).
    #[arg(long, value_name = "ORIGIN", num_args = 0..=1, default_missing_value = ingress::NODE_ORIGIN)]
    ingress: Option<String>,
}

/// The one mode clap accepted.
#[derive(Clone, Copy, Debug)]
pub enum Mode<'a> {
    /// The store opened in process by the shipped binary.
    Offline(&'a Path),
    /// The running deployment, reached through its ingress.
    Ingress(&'a str),
}

impl Target {
    /// The mode the parser accepted: `--store` for the offline path, the ingress (project node
    /// origin unless `--ingress` names another) otherwise.
    ///
    /// Clap's group makes the two flags mutually exclusive; a stray pair is a usage error rather
    /// than a panic, because it means the group and this call site have drifted apart, and the
    /// operator still needs the message that names both flags.
    pub fn mode(&self) -> Result<Mode<'_>> {
        match (self.store.as_deref(), self.ingress.as_deref()) {
            (Some(store), None) => Ok(Mode::Offline(store)),
            (Some(_), Some(_)) => bail!("--store <DIR> cannot be combined with --ingress <ORIGIN>"),
            (None, origin) => Ok(Mode::Ingress(origin.unwrap_or(ingress::NODE_ORIGIN))),
        }
    }
}

/// `cargo xtask census-status`: the store's per-table counters, counted by the service in ingress
/// mode and by `report --core` offline.
pub fn status(target: Target) -> Result<()> {
    match target.mode()? {
        Mode::Offline(store) => offline::report(store, Scope::Core),
        Mode::Ingress(origin) => service::status(origin),
    }
}

/// `cargo xtask coverage`: the census over every source.
pub fn coverage(target: Target) -> Result<()> {
    match target.mode()? {
        Mode::Offline(store) => offline::report(store, Scope::AllSources),
        Mode::Ingress(origin) => service::coverage(origin),
    }
}

/// `cargo xtask export`: the recruiting workbook built from evidence the store already holds.
pub fn export(
    target: Target,
    out: Option<&Path>,
    grad_year: i32,
    all_sources: bool,
    limit: Option<usize>,
) -> Result<()> {
    match target.mode()? {
        Mode::Offline(store) => offline::workbook(store, out, grad_year, all_sources, limit),
        Mode::Ingress(origin) => service::workbook(origin, out, grad_year, all_sources, limit),
    }
}
