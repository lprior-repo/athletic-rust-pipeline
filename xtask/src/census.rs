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

#[derive(clap::Subcommand, Debug)]
pub enum CensusCommand {
    #[command(
        about = "Print the core-scope census: the store's own counts from the running deployment (`Census/status`), or a whole core report offline (`census-service report --core`)"
    )]
    CensusStatus {
        #[command(flatten)]
        target: Target,
    },
    #[command(
        about = "Print the census over every source: `Report/run` on the running deployment, or `census-service report` offline"
    )]
    Coverage {
        #[command(flatten)]
        target: Target,
    },
    #[command(
        about = "Build the census workbook (`.xlsx`) and its text sidecars: `Workbook/run` on the running deployment, or `census-service workbook` offline"
    )]
    Export(ExportRequest),
}

impl CensusCommand {
    pub fn run(self) -> Result<()> {
        match self {
            Self::CensusStatus { target } => status(target),
            Self::Coverage { target } => coverage(target),
            Self::Export(request) => export(request),
        }
    }
}

#[derive(clap::Args, Debug)]
pub struct ExportRequest {
    #[command(flatten)]
    target: Target,
    #[arg(
        help = "Publication root the workbook generation is written under; the `.xlsx` lands at `<root>/current/workbook.xlsx` (defaults to `<store>/out/publication`)"
    )]
    #[arg(long, value_name = "DIR")]
    out: Option<PathBuf>,
    #[arg(help = "Graduation year used for the cohort sheets (2027 = the class of 2027)")]
    #[arg(long, default_value_t = 2027)]
    grad_year: i32,
    #[arg(
        help = "School year the workbook contact tenure and coach cells are assessed against (2026 = the 2026-27 school year)"
    )]
    #[arg(long)]
    school_year: i32,
    #[arg(
        help = "Reduce the best-results sheet over the core scope instead of every approved source"
    )]
    #[arg(long)]
    core: bool,
    #[arg(help = "Cap the per-athlete best-mark sheet at N rows")]
    #[arg(long, value_name = "N")]
    limit: Option<usize>,
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

pub fn export(request: ExportRequest) -> Result<()> {
    match request.target.mode()? {
        Mode::Offline(store) => offline::workbook(store, &request),
        Mode::Ingress(origin) => service::workbook(origin, &request),
    }
}
