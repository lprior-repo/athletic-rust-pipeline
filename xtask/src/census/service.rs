//! The ingress half of the census subcommands: the running deployment answers, the store is never
//! opened.
//!
//! Every function announces the exact handler it is about to call before calling it, so a returned
//! number can be traced to the service that produced it — and so the mode is visible in a transcript
//! rather than inferred from which flags were passed. The replies are read by field name: a name a
//! reply does not carry is an error rather than a zero, because a totals line that printed `0` for a
//! renamed field would read as an empty census.

use anyhow::{Context, Result};
use midwest_census::report::Scope;
use midwest_census::restate_services::{
    run_key, CensusIngressClient, ReportIngressClient, ReportReply, ReportRequest, StatusReply,
    WorkbookIngressClient, WorkbookReply, WorkbookRequest,
};
use midwest_census::Table;
use restate_sdk::prelude::*;
use serde_json::Value;
use std::path::Path;

use crate::ingress;

/// `Census/status`: the store's per-table counters, counted by the service.
pub(super) fn status(origin: &str) -> Result<()> {
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

/// `Report/run` over every source.
pub(super) fn coverage(origin: &str) -> Result<()> {
    let (endpoint, client) = ingress::job_client(origin)?;
    ingress::announce(&endpoint, "Report", "run");
    // The key names the job instance: a fresh instant per submission, so today's second report is a
    // second question rather than the first one's retained answer.
    let key = run_key("report", &[Scope::AllSources.as_str()]);
    let report = ReportIngressClient::from_client(client, key);
    let request = ReportRequest {
        scope: Some(Scope::AllSources.as_str().to_string()),
    };
    let response = ingress::block_on(report.run(Json(request)).call())?.map_err(ingress::error)?;
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

/// `Workbook/run` for the requested cohort.
pub(super) fn workbook(
    origin: &str,
    out: Option<&Path>,
    grad_year: i32,
    all_sources: bool,
    limit: Option<usize>,
) -> Result<()> {
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
    // The key names the job instance: a fresh instant per submission, so today's second workbook is
    // a second question rather than the first one's retained answer.
    let year = request
        .grad_year
        .map_or_else(|| "all".to_string(), |year| year.to_string());
    let key = run_key("workbook", &[&year, scope.as_str()]);
    let workbook = WorkbookIngressClient::from_client(client, key);
    let response =
        ingress::block_on(workbook.run(Json(request)).call())?.map_err(ingress::error)?;
    let Json(WorkbookReply { path, grad_year }) = response.into_body().map_err(ingress::error)?;
    println!("wrote {path}");
    match grad_year {
        Some(year) => println!("grad_year={year}"),
        None => println!("grad_year=all"),
    }
    Ok(())
}

/// The report totals line, in the labels the offline `report` prints for the same numbers.
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
