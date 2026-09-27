use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_reconcile::identity::Revision;
use census_report::report;
use census_service::restate_services::{
    run_key, BestsIngressClient, BestsReply, BestsRequest, ConsolidateIngressClient,
    ConsolidateReply, ConsolidateRequest, ConsolidatedTable, JurisdictionReport,
    JurisdictionRequest, ReportIngressClient, ReportReply, ReportRequest, WorkbookIngressClient,
    WorkbookReply, WorkbookRequest, DEFAULT_GENERATION,
};
use restate_sdk::prelude::*;

use super::national::{drive_jurisdiction, WorkflowFlags};
use census_service::ingress;

fn consolidate_client(origin: Option<&str>) -> Result<ConsolidateIngressClient<reqwest::Client>> {
    Ok(ConsolidateIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        run_key("consolidate", &[], DEFAULT_GENERATION),
    ))
}

fn report_client(
    origin: Option<&str>,
    scope: report::Scope,
) -> Result<ReportIngressClient<reqwest::Client>> {
    Ok(ReportIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        run_key("report", &[scope.as_str()], DEFAULT_GENERATION),
    ))
}

fn bests_client(
    origin: Option<&str>,
    scope: report::Scope,
    grad_year: Option<i16>,
    limit: Option<usize>,
) -> Result<BestsIngressClient<reqwest::Client>> {
    let year = grad_year.map_or_else(|| "all".to_string(), |year| year.to_string());
    let limit = limit.map_or_else(|| "all".to_string(), |limit| limit.to_string());
    Ok(BestsIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        run_key(
            "bests",
            &[scope.as_str(), &year, &limit],
            DEFAULT_GENERATION,
        ),
    ))
}

fn workbook_client(
    origin: Option<&str>,
    request: &WorkbookRequest,
) -> Result<WorkbookIngressClient<reqwest::Client>> {
    let year = request
        .grad_year
        .map_or_else(|| "all".to_string(), |year| year.to_string());
    let scope = request.scope.as_deref().unwrap_or("all");
    let limit = request
        .limit
        .map_or_else(|| "all".to_string(), |l| l.to_string());
    let out = request.out.as_deref().unwrap_or(".");
    Ok(WorkbookIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        run_key("workbook", &[&year, scope, &limit, out], DEFAULT_GENERATION),
    ))
}

pub(super) async fn consolidate(origin: Option<&str>) -> Result<Vec<ConsolidatedTable>> {
    let Json(ConsolidateReply { tables }) = consolidate_client(origin)?
        .run(Json(ConsolidateRequest::default()))
        .call()
        .await
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?;
    Ok(tables)
}

pub(super) struct ReportSummary {
    pub scope: String,
    pub json_path: String,
    pub csv_path: String,
    pub totals: serde_json::Value,
}

impl ReportSummary {
    pub(super) fn total(&self, field: &str) -> Result<u64> {
        self.totals
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .with_context(|| format!("the report reply carries no `{field}` total"))
    }
}

pub(super) async fn report(origin: Option<&str>, scope: report::Scope) -> Result<ReportSummary> {
    let Json(ReportReply {
        scope,
        json_path,
        csv_path,
        totals,
        generated_on: _,
    }) = report_client(origin, scope)?
        .run(Json(ReportRequest {
            scope: Some(scope.as_str().to_string()),
        }))
        .call()
        .await
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?;
    Ok(ReportSummary {
        scope,
        json_path,
        csv_path,
        totals,
    })
}

pub(super) async fn bests(
    origin: Option<&str>,
    scope: report::Scope,
    grad_year: Option<i16>,
    limit: Option<usize>,
) -> Result<BestsReply> {
    let request = BestsRequest {
        scope: Some(scope.as_str().to_string()),
        grad_year,
        limit,
    };
    let Json(reply) = bests_client(origin, scope, grad_year, limit)?
        .run(Json(request))
        .call()
        .await
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?;
    Ok(reply)
}

pub(super) async fn workbook(
    origin: Option<&str>,
    request: WorkbookRequest,
) -> Result<WorkbookReply> {
    let Json(reply) = workbook_client(origin, &request)?
        .run(Json(request))
        .call()
        .await
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?;
    Ok(reply)
}

pub(super) fn jurisdiction_request(
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    flags: &WorkflowFlags,
    refresh: bool,
    limit_per_state: Option<usize>,
    concurrency: usize,
    authorized_hosts: Vec<String>,
) -> JurisdictionRequest {
    JurisdictionRequest {
        jurisdiction,
        season,
        revision: Revision(flags.revision),
        refresh,
        limit_per_state,
        concurrency,
        observed_on: None,
        authorized_hosts,
    }
}

pub(super) async fn drive_states(
    origin: &str,
    requests: Vec<JurisdictionRequest>,
    rounds: u64,
    line: impl Fn(&JurisdictionReport) -> Result<String>,
) -> Result<()> {
    let ingestion = ingress::client(origin)?;
    for request in requests {
        let report = drive_jurisdiction(&ingestion, request, rounds).await?;
        println!("{}", line(&report)?);
    }
    Ok(())
}
