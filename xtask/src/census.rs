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
//! Both modes announce what they ran before running it — the offline mode through [`Cmd`]'s
//! `+ <command>` line, the ingress mode through a `+ POST <origin>restate/call/<Service>/<handler>`
//! line — and both print the same operator-facing lines from what came back.

use anyhow::{bail, Context, Result};
use midwest_census::report::Scope;
use midwest_census::restate_services::{
    run_key, CensusIngressClient, ReportIngressClient, ReportReply, ReportRequest, StatusReply,
    WorkbookIngressClient, WorkbookReply, WorkbookRequest,
};
use midwest_census::Table;
use restate_sdk::prelude::*;
use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::cmd::Cmd;
use crate::ingress;

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
        Mode::Offline(store) => report_offline(store, Scope::Core),
        Mode::Ingress(origin) => {
            let (endpoint, client) = ingress::client(origin)?;
            ingress::announce(&endpoint, "Census", "status");
            let census = CensusIngressClient::from_client(client);
            let response = ingress::block_on(census.status().call())?.map_err(ingress::error)?;
            let Json(status) = response.into_body().map_err(ingress::error)?;
            println!(
                "store status: schools={} athletes={} observations={} bytes_on_disk={} today={}",
                rows(&status, Table::Schools),
                rows(&status, Table::Athletes),
                status.observations,
                status.bytes_on_disk,
                status.today
            );
            Ok(())
        }
    }
}

/// `cargo xtask coverage`: the census over every source.
pub fn coverage(target: Target) -> Result<()> {
    match target.mode()? {
        Mode::Offline(store) => report_offline(store, Scope::AllSources),
        Mode::Ingress(origin) => {
            let (endpoint, client) = ingress::job_client(origin)?;
            ingress::announce(&endpoint, "Report", "run");
            // The key names the job instance: a fresh instant per submission, so today's second
            // report is a second question rather than the first one's retained answer.
            let key = run_key("report", &[Scope::AllSources.as_str()]);
            let report = ReportIngressClient::from_client(client, key);
            let request = ReportRequest {
                scope: Some(Scope::AllSources.as_str().to_string()),
            };
            let response =
                ingress::block_on(report.run(Json(request)).call())?.map_err(ingress::error)?;
            let Json(ReportReply {
                scope,
                totals,
                json_path,
                csv_path,
                ..
            }) = response.into_body().map_err(ingress::error)?;
            println!("wrote {json_path}");
            println!("wrote {csv_path}");
            totals_line(&scope, &totals)
        }
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
        Mode::Offline(store) => workbook_offline(store, out, grad_year, all_sources, limit),
        Mode::Ingress(origin) => {
            let (endpoint, client) = ingress::job_client(origin)?;
            ingress::announce(&endpoint, "Workbook", "run");
            let scope = if all_sources {
                Scope::AllSources
            } else {
                Scope::Core
            };
            let request = WorkbookRequest {
                grad_year: Some(school_year(grad_year)?),
                limit,
                scope: Some(scope.as_str().to_string()),
                out: out.map(|path| path.display().to_string()),
            };
            // The key names the job instance: a fresh instant per submission, so today's second
            // workbook is a second question rather than the first one's retained answer.
            let year = request
                .grad_year
                .map_or_else(|| "all".to_string(), |year| year.to_string());
            let key = run_key("workbook", &[&year, scope.as_str()]);
            let workbook = WorkbookIngressClient::from_client(client, key);
            let response =
                ingress::block_on(workbook.run(Json(request)).call())?.map_err(ingress::error)?;
            let Json(WorkbookReply { path, grad_year }) =
                response.into_body().map_err(ingress::error)?;
            println!("wrote {path}");
            match grad_year {
                Some(year) => println!("grad_year={year}"),
                None => println!("grad_year=all"),
            }
            Ok(())
        }
    }
}

/// The offline path: `midwest-census report`, with the store opened in process.
fn report_offline(store: &Path, scope: Scope) -> Result<()> {
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

/// The offline path for the workbook: `midwest-census workbook` on the same store.
fn workbook_offline(
    store: &Path,
    out: Option<&Path>,
    grad_year: i32,
    all_sources: bool,
    limit: Option<usize>,
) -> Result<()> {
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
        .args(["workbook", "--grad-year", &grad_year.to_string()]);
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

/// The report totals line, in the labels the offline `report` prints for the same numbers.
///
/// The reply carries the totals as the census document publishes them, so the fields are read by
/// name; a name the reply does not carry is an error rather than a zero, because a totals line that
/// printed `0` for a renamed field would read as an empty census.
fn totals_line(scope: &str, totals: &Value) -> Result<()> {
    println!(
        "scope={scope} totals: schools={} athletes={} co2027={} (boys={} girls={}) profile_url={} multisource={} coaches={}",
        count(totals, "schools")?,
        count(totals, "athletes")?,
        count(totals, "class_of_2027")?,
        count(totals, "class_of_2027_boys")?,
        count(totals, "class_of_2027_girls")?,
        count(totals, "class_of_2027_with_profile_url")?,
        count(totals, "class_of_2027_multisource")?,
        count(totals, "coaches")?,
    );
    Ok(())
}

/// One totals field from the wire reply.
fn count(totals: &Value, key: &str) -> Result<u64> {
    totals
        .get(key)
        .and_then(Value::as_u64)
        .with_context(|| format!("the report reply's totals carry no `{key}` count"))
}

/// One table's row count as `Census/status` reports it.
///
/// The handler lists every table it knows, including the ones holding no rows, so a miss here means
/// the deployment does not have that table at all rather than that it is empty.
fn rows(status: &StatusReply, table: Table) -> u64 {
    status
        .tables
        .iter()
        .find(|count| count.table == table.file())
        .map_or(0, |count| count.rows)
}

/// Cohort fields are `i16`; the flag is an `i32` so the offline child reports its own range error,
/// and this conversion has to reproduce that message for the wire request.
fn school_year(grad_year: i32) -> Result<i16> {
    i16::try_from(grad_year)
        .with_context(|| format!("--grad-year {grad_year} is not a representable year"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// A parser wrapper, because `Target` is a flattened argument group rather than a command.
    #[derive(clap::Parser)]
    struct Wrapper {
        #[command(flatten)]
        target: Target,
    }

    /// The contract the goal's verification commands rely on: the flag-free form asks the running
    /// deployment at the project node, `--store` selects the offline path instead, and a pair is
    /// refused rather than silently preferring one of them.
    #[test]
    fn a_flag_free_invocation_uses_the_project_node_and_a_store_flag_goes_offline() {
        let flag_free = Wrapper::try_parse_from(["xtask"]).expect("flag-free invocation parses");
        match flag_free.target.mode().expect("mode") {
            Mode::Ingress(origin) => assert_eq!(origin, ingress::NODE_ORIGIN),
            Mode::Offline(path) => panic!("expected the ingress path, got offline {path:?}"),
        }

        let offline =
            Wrapper::try_parse_from(["xtask", "--store", "/tmp/store"]).expect("store parses");
        match offline.target.mode().expect("mode") {
            Mode::Offline(path) => assert_eq!(path, std::path::Path::new("/tmp/store")),
            Mode::Ingress(origin) => panic!("expected the offline path, got ingress {origin}"),
        }

        let both = Wrapper::try_parse_from(["xtask", "--store", "/tmp/store", "--ingress"]);
        assert!(both.is_err(), "both flags must be refused by the parser");
    }
}
