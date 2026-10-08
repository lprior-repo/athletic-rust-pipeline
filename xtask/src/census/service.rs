use anyhow::{Context, Result};
use census_report::report::Scope;
use census_service::restate_services::{
    run_key, workbook_request_key, CensusIngressClient, ReportIngressClient, ReportReply,
    ReportRequest, StatusReply, WorkbookIngressClient, WorkbookReply, WorkbookRequest,
};
use census_store::Table;
use restate_sdk::prelude::*;
use serde_json::Value;

use crate::ingress;

use super::ExportRequest;

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

pub(super) fn coverage(origin: &str) -> Result<()> {
    let (endpoint, client) = ingress::job_client(origin)?;
    ingress::announce(&endpoint, "Report", "run");
    let key = run_key("report", &[Scope::AllSources.as_str()], "1");
    let report = ReportIngressClient::from_client(client, key);
    let request = ReportRequest {
        scope: Some(Scope::AllSources.as_str().to_string()),
    };
    let response = ingress::block_on(report.run(Json(request)).call())?.map_err(ingress::error)?;
    let Json(reply) = response.into_body().map_err(ingress::error)?;
    print_report(reply)
}

fn print_report(reply: ReportReply) -> Result<()> {
    let ReportReply {
        scope,
        totals,
        json_path,
        csv_path,
        ..
    } = reply;
    println!("wrote {json_path}");
    println!("wrote {csv_path}");
    totals_line(&scope, &totals)
}

pub(super) fn workbook(origin: &str, options: &ExportRequest) -> Result<()> {
    let (endpoint, client) = ingress::job_client(origin)?;
    ingress::announce(&endpoint, "Workbook", "run");
    let request = workbook_request(options)?;
    let key = workbook_request_key(&request);
    let workbook = WorkbookIngressClient::from_client(client, key);
    let response =
        ingress::block_on(workbook.run(Json(request)).call())?.map_err(ingress::error)?;
    let Json(reply) = response.into_body().map_err(ingress::error)?;
    print_workbook(reply);
    Ok(())
}

fn workbook_request(options: &ExportRequest) -> Result<WorkbookRequest> {
    let scope = if options.core {
        Scope::Core
    } else {
        Scope::AllSources
    };
    Ok(WorkbookRequest {
        grad_year: Some(grad_year_i16(options.grad_year)?),
        limit: options.limit,
        scope: Some(scope.as_str().to_string()),
        out: options
            .out
            .as_deref()
            .map(|path| path.display().to_string()),
        school_year: Some(school_year_i16(options.school_year)?),
    })
}

fn print_workbook(WorkbookReply { path, grad_year }: WorkbookReply) {
    println!("wrote {path}");
    match grad_year {
        Some(year) => println!("grad_year={year}"),
        None => println!("grad_year=all"),
    }
}

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

fn count(totals: &Value, key: &str) -> Result<u64> {
    totals
        .get(key)
        .and_then(Value::as_u64)
        .with_context(|| format!("the report reply's totals carry no `{key}` count"))
}

fn rows(status: &StatusReply, table: Table) -> u64 {
    status
        .tables
        .iter()
        .find(|count| count.table == table.file())
        .map_or(0, |count| count.rows)
}

fn year_i16(label: &str, year: i32) -> Result<i16> {
    i16::try_from(year).with_context(|| format!("{label} {year} is not a representable year"))
}

fn school_year_i16(year: i32) -> Result<i16> {
    year_i16("--school-year", year)
}

fn grad_year_i16(year: i32) -> Result<i16> {
    year_i16("--grad-year", year)
}
