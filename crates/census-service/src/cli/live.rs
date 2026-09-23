//! The client side of the live path: the ingress submissions the pipeline commands share.
//!
//! The commands own their flags and their output, this owns the wire calls those flags turn into, so
//! `run`'s stages and the single-stage commands cannot drift apart about a request's shape. Nothing
//! here opens a store: every call addresses the running census service, which holds the store's single
//! writer.
//!
//! One `Census` client serves a whole command — a run submits several stages — and the per-state
//! jurisdiction calls are [`drive_states`], which submits one object per state and waits for each in
//! turn so the operator's output stays ordered.

use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_service::census::Revision;
use census_report::report;
use census_service::restate_services::{
    run_key, BestsIngressClient, BestsReply, BestsRequest, ConsolidateIngressClient,
    ConsolidateReply, ConsolidateRequest, ConsolidatedTable, JurisdictionReport,
    JurisdictionRequest, ReportIngressClient, ReportReply, ReportRequest, WorkbookIngressClient,
    WorkbookReply, WorkbookRequest,
};
use restate_sdk::prelude::*;

use super::national::{drive_jurisdiction, WorkflowFlags};
use census_service::ingress;

/// The `Consolidate` workflow's ingress client for `origin`, under today's key.
fn consolidate_client(origin: Option<&str>) -> Result<ConsolidateIngressClient<reqwest::Client>> {
    Ok(ConsolidateIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        run_key("consolidate", &[]),
    ))
}

/// The `Report` workflow's ingress client for `origin`, under today's key for `scope`.
fn report_client(
    origin: Option<&str>,
    scope: report::Scope,
) -> Result<ReportIngressClient<reqwest::Client>> {
    Ok(ReportIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        run_key("report", &[scope.as_str()]),
    ))
}

/// The `Bests` workflow's ingress client for `origin`, under today's key for the whole request.
///
/// The cohort and the limit are part of the key: a different cohort or a different limit is a
/// different answer, not a resubmission of this one.
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
        run_key("bests", &[scope.as_str(), &year, &limit]),
    ))
}

/// The `Workbook` workflow's ingress client for `origin`, under today's key for the request.
fn workbook_client(
    origin: Option<&str>,
    request: &WorkbookRequest,
) -> Result<WorkbookIngressClient<reqwest::Client>> {
    let year = request
        .grad_year
        .map_or_else(|| "all".to_string(), |year| year.to_string());
    let scope = request.scope.as_deref().unwrap_or("all");
    Ok(WorkbookIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        run_key("workbook", &[&year, scope]),
    ))
}

/// Merge every table's append observations in the running service.
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

/// One scope's census as the service built it, with the totals its callers print.
pub(super) struct ReportSummary {
    pub scope: String,
    pub json_path: String,
    pub csv_path: String,
    pub totals: serde_json::Value,
}

impl ReportSummary {
    /// One total out of the reply's JSON.
    ///
    /// The reply is built from the same totals model the offline path prints, so a field the model
    /// always carries is a wire fault when it is missing — not a zero to print.
    pub(super) fn total(&self, field: &str) -> Result<u64> {
        self.totals
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .with_context(|| format!("the report reply carries no `{field}` total"))
    }
}

/// Build one scope's census in the running service.
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

/// Reduce one cohort's best marks in the running service.
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

/// Write the census workbook in the running service.
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

/// The request one state's jurisdiction object is asked to run.
///
/// The object's key is the jurisdiction identity, and the request repeats those three fields so the
/// handler can refuse one routed to the wrong key — which is why the season comes from the command's
/// own year flag and the revision from [`WorkflowFlags`].
pub(super) fn jurisdiction_request(
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    flags: &WorkflowFlags,
    refresh: bool,
    limit_per_state: Option<usize>,
    concurrency: usize,
) -> JurisdictionRequest {
    JurisdictionRequest {
        jurisdiction,
        season,
        revision: Revision(flags.revision),
        refresh,
        limit_per_state,
        concurrency,
        observed_on: None,
    }
}

/// Submit one jurisdiction object per request, waiting for each and printing `line` for it.
///
/// The submission line, the deduplication note and the per-state summary are all the operator sees of
/// a run that may take hours: one state is waited for before the next is submitted, so the output
/// reads in the order the states were asked for rather than in completion order.
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
