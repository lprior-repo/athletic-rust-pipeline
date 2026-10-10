use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_reconcile::identity::Revision;
use census_report::report;
use census_service::restate_services::{
    bests_export_key, consolidate_export_key, report_export_key, workbook_request_key,
    BestsIngressClient, BestsReply, BestsRequest, ConsolidateIngressClient, ConsolidateReply,
    ConsolidateRequest, ConsolidatedTable, ExportGeneration, JurisdictionReport,
    JurisdictionRequest, ReportIngressClient, ReportReply, ReportRequest, WorkbookIngressClient,
    WorkbookReply, WorkbookRequest,
};
use restate_sdk::prelude::*;

use super::national::{drive_jurisdiction, WorkflowFlags};
use census_crawl::ingress::CurrentRoute;
use census_service::ingress;

fn log_submit(message: &str, generation: &ExportGeneration, key: &str) {
    tracing::info!(generation = generation.as_str(), key = key, "{message}");
}

fn consolidate_client(
    origin: Option<&str>,
    key: String,
) -> Result<ConsolidateIngressClient<CurrentRoute>> {
    Ok(ConsolidateIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        key,
    ))
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

pub(super) async fn consolidate(
    origin: Option<&str>,
    generation: &ExportGeneration,
) -> Result<Vec<ConsolidatedTable>> {
    let key = consolidate_export_key(generation);
    log_submit("submitting consolidate export", generation, key.as_str());
    let Json(ConsolidateReply { tables }) = consolidate_client(origin, key)?
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
    pub generation: String,
    pub generated_on: String,
}

impl ReportSummary {
    pub(super) fn total(&self, field: &str) -> Result<u64> {
        self.totals
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .with_context(|| format!("the report reply carries no `{field}` total"))
    }
}

pub(super) async fn report(
    origin: Option<&str>,
    scope: report::Scope,
    generation: &ExportGeneration,
) -> Result<ReportSummary> {
    let key = report_export_key(scope.as_str(), generation)?;
    log_submit("submitting report export", generation, key.as_str());
    let Json(reply) = report_client(origin, key)?
        .run(Json(ReportRequest {
            scope: Some(scope.as_str().to_string()),
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
) -> BestsRequest {
    BestsRequest {
        scope: Some(scope.as_str().to_string()),
        grad_year,
        limit,
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
    generation: &ExportGeneration,
) -> Result<BestsReply> {
    let year = grad_year.map_or_else(|| "all".to_string(), |year| year.to_string());
    let limit_text = limit.map_or_else(|| "all".to_string(), |limit| limit.to_string());
    let key = bests_export_key(scope.as_str(), &year, &limit_text, generation)?;
    log_submit("submitting bests export", generation, key.as_str());
    let request = bests_request(scope, grad_year, limit);
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
    for request in requests {
        let report = drive_jurisdiction(&ingestion, request, rounds).await?;
        println!("{}", line(&report)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_service::restate_services::reject_offline_generation;
    use census_service::restate_services::DEFAULT_GENERATION;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn the_default_generation_matches_the_legacy_constant() -> TestResult {
        assert_eq!(
            ExportGeneration::default_generation().as_str(),
            DEFAULT_GENERATION
        );
        assert_eq!(
            ExportGeneration::default_generation(),
            ExportGeneration::parse(DEFAULT_GENERATION)?
        );
        Ok(())
    }

    #[test]
    fn identical_semantic_requests_with_one_generation_share_retry_keys() -> TestResult {
        let generation = ExportGeneration::default_generation();
        assert_eq!(
            report_export_key("core", &generation)?,
            report_export_key("core", &generation)?
        );
        assert_eq!(
            report_export_key("core", &generation)?.as_str(),
            "report:core:1"
        );
        assert_eq!(
            bests_export_key("all", "2027", "50", &generation)?,
            bests_export_key("all", "2027", "50", &generation)?
        );
        assert_eq!(
            consolidate_export_key(&generation),
            consolidate_export_key(&generation)
        );
        assert_eq!(
            consolidate_export_key(&generation).as_str(),
            "consolidate:1"
        );
        Ok(())
    }

    #[test]
    fn a_fresh_generation_produces_fresh_export_keys() -> TestResult {
        let previous = ExportGeneration::default_generation();
        let next = ExportGeneration::parse("2")?;
        assert_ne!(
            report_export_key("core", &previous)?,
            report_export_key("core", &next)?
        );
        assert_eq!(report_export_key("core", &next)?.as_str(), "report:core:2");
        assert_ne!(
            bests_export_key("core", "2027", "all", &previous)?,
            bests_export_key("core", "2027", "all", &next)?
        );
        assert_ne!(
            consolidate_export_key(&previous),
            consolidate_export_key(&next)
        );
        Ok(())
    }

    #[test]
    fn export_generations_are_bounded_to_key_safe_text() -> TestResult {
        assert!(ExportGeneration::parse("").is_err());
        assert!(ExportGeneration::parse(&"9".repeat(33)).is_err());
        assert!(ExportGeneration::parse(&"9".repeat(32)).is_ok());
        for refused in ["a/b", "a b", "a:b", "gén", "a+b", "a*b"] {
            assert!(
                ExportGeneration::parse(refused).is_err(),
                "{refused} must not become an export generation"
            );
        }
        for allowed in ["1", "2", "2026-10-10", "run_7.final", "abc-def_1.2"] {
            assert!(
                ExportGeneration::parse(allowed).is_ok(),
                "{allowed} must stay a selectable export generation"
            );
        }
        Ok(())
    }

    #[test]
    fn workbook_keys_follow_the_selected_generation() -> TestResult {
        let request = WorkbookRequest {
            grad_year: Some(2027),
            limit: Some(50),
            scope: Some("core".to_string()),
            out: Some("/tmp/root-a".to_string()),
            school_year: Some(2026),
        };
        let legacy = ExportGeneration::default_generation();
        assert_eq!(
            workbook_request_key(&request, &legacy)?.as_str(),
            "workbook:2027:core:50:/tmp/root-a:2026:1"
        );
        let next = ExportGeneration::parse("2")?;
        assert_eq!(
            workbook_request_key(&request, &next)?.as_str(),
            "workbook:2027:core:50:/tmp/root-a:2026:2"
        );
        assert_ne!(
            workbook_request_key(&request, &legacy)?,
            workbook_request_key(&request, &next)?,
            "a fresh generation must not reattach the legacy workflow"
        );
        Ok(())
    }

    #[test]
    fn the_generation_resolver_defaults_without_a_flag() -> TestResult {
        assert_eq!(
            ExportGeneration::resolve(None)?.as_str(),
            DEFAULT_GENERATION
        );
        assert_eq!(ExportGeneration::resolve(Some("7"))?.as_str(), "7");
        assert!(ExportGeneration::resolve(Some("bad/generation")).is_err());
        Ok(())
    }

    #[test]
    fn export_keys_refuse_colon_parts_on_every_builder() -> TestResult {
        let generation = ExportGeneration::default_generation();
        check!(
            report_export_key("co:re", &generation).is_err(),
            "a colon scope must not shift the report key segments"
        );
        check!(
            report_export_key("", &generation).is_err(),
            "an empty scope must not collapse the report key grammar"
        );
        check!(
            bests_export_key("a:ll", "2027", "50", &generation).is_err(),
            "a colon scope must not shift the bests key segments"
        );
        check!(
            bests_export_key("all", "20:27", "50", &generation).is_err(),
            "a colon year must not shift the bests key segments"
        );
        check!(
            bests_export_key("all", "2027", "5:0", &generation).is_err(),
            "a colon limit must not shift the bests key segments"
        );
        check!(
            bests_export_key("all", "2027", "", &generation).is_err(),
            "an empty limit must not collapse the bests key grammar"
        );
        Ok(())
    }

    #[test]
    fn offline_routes_refuse_an_explicit_generation() -> TestResult {
        assert!(reject_offline_generation(None).is_ok());
        let refused = reject_offline_generation(Some("2"));
        check!(
            refused.is_err(),
            "an offline route must not silently ignore a requested generation"
        );
        Ok(())
    }
}
