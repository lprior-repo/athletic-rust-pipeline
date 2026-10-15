use anyhow::{bail, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_reconcile::identity::Revision;
use census_report::report;
use census_service::restate_services::{
    bests_export_key, report_export_key, workbook_request_key, BestsIngressClient, BestsReply,
    BestsRequest, ExportGeneration, JurisdictionReport, JurisdictionRequest, ReportIngressClient,
    ReportReply, ReportRequest, WorkbookIngressClient, WorkbookReply, WorkbookRequest,
};
use restate_sdk::prelude::*;

use super::national::{drive_jurisdiction, WorkflowFlags};
use census_crawl::ingress::CurrentRoute;
use census_service::ingress;

fn log_submit(message: &str, generation: &ExportGeneration, key: &str) {
    tracing::info!(generation = generation.as_str(), key = key, "{message}");
}

fn report_client(origin: Option<&str>, key: String) -> Result<ReportIngressClient<CurrentRoute>> {
    Ok(ReportIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        key,
    ))
}

fn bests_client(origin: Option<&str>, key: String) -> Result<BestsIngressClient<CurrentRoute>> {
    Ok(BestsIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        key,
    ))
}

fn workbook_client(
    origin: Option<&str>,
    key: String,
) -> Result<WorkbookIngressClient<CurrentRoute>> {
    Ok(WorkbookIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        key,
    ))
}

pub(super) struct ReportSummary {
    pub scope: String,
    pub json_path: String,
    pub csv_path: String,
    pub totals: serde_json::Value,
    pub generation: String,
    pub generated_on: String,
}

pub(super) async fn report(
    origin: Option<&str>,
    scope: report::Scope,
    jurisdictions: &[census_domain::UsJurisdiction],
    generation: &ExportGeneration,
) -> Result<ReportSummary> {
    let key = report_export_key(scope.as_str(), jurisdictions, generation)?;
    log_submit("submitting report export", generation, key.as_str());
    let Json(reply) = report_client(origin, key)?
        .run(Json(ReportRequest {
            scope: Some(scope.as_str().to_string()),
            jurisdictions: jurisdictions.to_vec(),
        }))
        .call()
        .await
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?;
    Ok(summarize_report(reply, generation))
}

fn bests_request(
    scope: report::Scope,
    grad_year: Option<i16>,
    limit: Option<usize>,
    jurisdictions: &[census_domain::UsJurisdiction],
) -> BestsRequest {
    BestsRequest {
        scope: Some(scope.as_str().to_string()),
        grad_year,
        limit,
        jurisdictions: jurisdictions.to_vec(),
    }
}

fn summarize_report(reply: ReportReply, generation: &ExportGeneration) -> ReportSummary {
    let ReportReply {
        scope,
        json_path,
        csv_path,
        totals,
        generated_on,
    } = reply;
    let summary = ReportSummary {
        scope,
        json_path,
        csv_path,
        totals,
        generation: generation.as_str().to_string(),
        generated_on,
    };
    tracing::info!(
        generation = summary.generation.as_str(),
        generated_on = summary.generated_on.as_str(),
        scope = summary.scope.as_str(),
        json_path = summary.json_path.as_str(),
        "report export complete"
    );
    summary
}

pub(super) async fn bests(
    origin: Option<&str>,
    scope: report::Scope,
    grad_year: Option<i16>,
    limit: Option<usize>,
    jurisdictions: &[census_domain::UsJurisdiction],
    generation: &ExportGeneration,
) -> Result<BestsReply> {
    let year = grad_year.map_or_else(|| "all".to_string(), |year| year.to_string());
    let limit_text = limit.map_or_else(|| "all".to_string(), |limit| limit.to_string());
    let key = bests_export_key(
        scope.as_str(),
        &year,
        &limit_text,
        jurisdictions,
        generation,
    )?;
    log_submit("submitting bests export", generation, key.as_str());
    let request = bests_request(scope, grad_year, limit, jurisdictions);
    let Json(reply) = bests_client(origin, key)?
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
    generation: &ExportGeneration,
) -> Result<WorkbookReply> {
    let key = workbook_request_key(&request, generation)?;
    log_submit("submitting workbook export", generation, key.as_str());
    let Json(reply) = workbook_client(origin, key)?
        .run(Json(request))
        .call()
        .await
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?;
    Ok(reply)
}

pub(super) struct JurisdictionRun {
    pub refresh: bool,
    pub limit_per_state: Option<usize>,
    pub concurrency: usize,
    pub authorized_hosts: Vec<String>,
    pub source_parallelism: usize,
}

pub(super) fn jurisdiction_request(
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    flags: &WorkflowFlags,
    options: JurisdictionRun,
) -> Result<JurisdictionRequest> {
    Ok(JurisdictionRequest {
        jurisdiction,
        season,
        revision: Revision(flags.revision),
        history: census_service::restate_services::HistoryWindow::cohort(
            &census_crawl::net::today_iso(),
        )?,
        refresh: options.refresh,
        limit_per_state: options.limit_per_state,
        concurrency: options.concurrency,
        observed_on: None,
        authorized_hosts: options.authorized_hosts,
        source_parallelism: options.source_parallelism,
    })
}

pub(super) async fn drive_states(
    origin: &str,
    requests: Vec<JurisdictionRequest>,
    rounds: u64,
    line: impl Fn(&JurisdictionReport) -> Result<String>,
) -> Result<()> {
    let ingestion = ingress::client(origin)?;
    let mut failed = Vec::new();
    for request in requests {
        let report = drive_jurisdiction(&ingestion, request, rounds).await?;
        println!("{}", line(&report)?);
        if report
            .plan
            .refused
            .iter()
            .any(|r| r.reason != "no run stage sweeps this source per jurisdiction")
            || !report.rosters.errors.is_empty()
            || report.rosters.blocked
        {
            failed.push(report.jurisdiction.code().to_string());
        }
    }
    if failed.is_empty() {
        return Ok(());
    }
    bail!(
        "team collection did not complete: failed [{}]",
        failed.join(", ")
    )
}

#[cfg(test)]
#[path = "live_tests.rs"]
mod tests;
