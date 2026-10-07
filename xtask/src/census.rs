use anyhow::{bail, Result};
use census_report::report::Scope;
use std::path::{Path, PathBuf};

use crate::ingress;

mod offline;
mod service;

#[cfg(test)]
#[path = "census/tests.rs"]
mod tests;

#[derive(clap::Args, Debug)]
#[command(
    about = "Where a census subcommand reads from: the store directly, or the running deployment",
    long_about = "Where a census subcommand reads from: the store directly, or the running deployment.\n\n`--store` selects the offline mode; without it the command submits to the running deployment, so the flag-free form works exactly while `census-serve` holds the store. The two are exclusive by construction: `--store` opens the store in process, which is only possible with the worker stopped, and the ingress asks the worker that holds it."
)]
#[group(multiple = false)]
pub struct Target {
    #[arg(
        help = "Store root (HTTP cache, journals, entity logs, output snapshots). Needs `census-serve` stopped: the store is single-writer"
    )]
    #[arg(long, value_name = "DIR")]
    store: Option<PathBuf>,
    #[arg(
        help = "Restate ingress origin of the running deployment; `--ingress` alone uses the project node origin (http://127.0.0.1:18095/)"
    )]
    #[arg(long, value_name = "ORIGIN", num_args = 0..=1, default_missing_value = ingress::NODE_ORIGIN)]
    ingress: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum Mode<'a> {
    Offline(&'a Path),
    Ingress(&'a str),
}

impl Target {
    pub fn mode(&self) -> Result<Mode<'_>> {
        match (self.store.as_deref(), self.ingress.as_deref()) {
            (Some(store), None) => Ok(Mode::Offline(store)),
            (Some(_), Some(_)) => bail!("--store <DIR> cannot be combined with --ingress <ORIGIN>"),
            (None, origin) => Ok(Mode::Ingress(
                origin.map_or(ingress::NODE_ORIGIN, core::convert::identity),
            )),
        }
    }
}

pub fn status(target: Target) -> Result<()> {
    match target.mode()? {
        Mode::Offline(store) => offline::report(store, Scope::Core),
        Mode::Ingress(origin) => service::status(origin),
    }
}

pub fn coverage(target: Target) -> Result<()> {
    match target.mode()? {
        Mode::Offline(store) => offline::report(store, Scope::AllSources),
        Mode::Ingress(origin) => service::coverage(origin),
    }
}

pub fn export(
    target: Target,
    out: Option<&Path>,
    grad_year: i32,
    school_year: i32,
    core: bool,
    limit: Option<usize>,
) -> Result<()> {
    match target.mode()? {
        Mode::Offline(store) => offline::workbook(store, out, grad_year, school_year, core, limit),
        Mode::Ingress(origin) => {
            service::workbook(origin, out, grad_year, school_year, core, limit)
        }
    }
}
