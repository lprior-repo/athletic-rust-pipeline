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

fn export_generation() -> Result<ExportGeneration> {
    match std::env::var("CENSUS_EXPORT_GENERATION") {
        Err(std::env::VarError::NotPresent) => Ok(ExportGeneration::default_generation()),
        Err(error) => Err(error).with_context(|| "reading CENSUS_EXPORT_GENERATION"),
        Ok(value) => ExportGeneration::parse(&value)
            .with_context(|| format!("parsing CENSUS_EXPORT_GENERATION {value:?}")),
    }
}

fn consolidate_client(
    origin: Option<&str>,
    generation: &ExportGeneration,
) -> Result<ConsolidateIngressClient<CurrentRoute>> {
    Ok(ConsolidateIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        consolidate_export_key(generation),
    ))
}

fn report_client(
    origin: Option<&str>,
    scope: report::Scope,
    generation: &ExportGeneration,
) -> Result<ReportIngressClient<CurrentRoute>> {
    Ok(ReportIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        report_export_key(scope.as_str(), generation),
    ))
}

fn bests_client(
    origin: Option<&str>,
    scope: report::Scope,
    grad_year: Option<i16>,
    limit: Option<usize>,
    generation: &ExportGeneration,
) -> Result<BestsIngressClient<CurrentRoute>> {
    let year = grad_year.map_or_else(|| "all".to_string(), |year| year.to_string());
    let limit = limit.map_or_else(|| "all".to_string(), |limit| limit.to_string());
    Ok(BestsIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        bests_export_key(scope.as_str(), &year, &limit, generation),
    ))
}

fn workbook_client(
    origin: Option<&str>,
    request: &WorkbookRequest,
) -> Result<WorkbookIngressClient<CurrentRoute>> {
    Ok(WorkbookIngressClient::from_client(
        ingress::job_client(ingress::origin(origin))?,
        workbook_request_key(request),
    ))
}

pub(super) async fn consolidate(origin: Option<&str>) -> Result<Vec<ConsolidatedTable>> {
    consolidate_with_generation(origin, &export_generation()?).await
}

pub(super) async fn consolidate_with_generation(
    origin: Option<&str>,
    generation: &ExportGeneration,
) -> Result<Vec<ConsolidatedTable>> {
    tracing::info!(
        generation = generation.as_str(),
        key = consolidate_export_key(generation).as_str(),
        "submitting consolidate export"
    );
    let Json(ConsolidateReply { tables }) = consolidate_client(origin, generation)?
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

pub(super) async fn report(origin: Option<&str>, scope: report::Scope) -> Result<ReportSummary> {
    report_with_generation(origin, scope, &export_generation()?).await
}

pub(super) async fn report_with_generation(
    origin: Option<&str>,
    scope: report::Scope,
    generation: &ExportGeneration,
) -> Result<ReportSummary> {
    tracing::info!(
        generation = generation.as_str(),
        key = report_export_key(scope.as_str(), generation).as_str(),
        "submitting report export"
    );
    let Json(ReportReply {
        scope,
        json_path,
        csv_path,
        totals,
        generated_on,
    }) = report_client(origin, scope, generation)?
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
        generation: generation.as_str().to_string(),
        generated_on,
    })
    .inspect(|summary| {
        tracing::info!(
            generation = summary.generation.as_str(),
            generated_on = summary.generated_on.as_str(),
            scope = summary.scope.as_str(),
            json_path = summary.json_path.as_str(),
            "report export complete"
        );
    })
}

pub(super) async fn bests(
    origin: Option<&str>,
    scope: report::Scope,
    grad_year: Option<i16>,
    limit: Option<usize>,
) -> Result<BestsReply> {
    bests_with_generation(origin, scope, grad_year, limit, &export_generation()?).await
}

pub(super) async fn bests_with_generation(
    origin: Option<&str>,
    scope: report::Scope,
    grad_year: Option<i16>,
    limit: Option<usize>,
    generation: &ExportGeneration,
) -> Result<BestsReply> {
    let year = grad_year.map_or_else(|| "all".to_string(), |year| year.to_string());
    let limit_text = limit.map_or_else(|| "all".to_string(), |limit| limit.to_string());
    tracing::info!(
        generation = generation.as_str(),
        key = bests_export_key(scope.as_str(), &year, &limit_text, generation).as_str(),
        "submitting bests export"
    );
    let request = BestsRequest {
        scope: Some(scope.as_str().to_string()),
        grad_year,
        limit,
    };
    let Json(reply) = bests_client(origin, scope, grad_year, limit, generation)?
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
    use census_service::restate_services::DEFAULT_GENERATION;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    struct ExportGenerationGuard {
        previous: Option<String>,
    }

    impl ExportGenerationGuard {
        fn set(value: &str) -> Self {
            let previous = std::env::var("CENSUS_EXPORT_GENERATION").ok();
            std::env::set_var("CENSUS_EXPORT_GENERATION", value);
            Self { previous }
        }

        fn clear() -> Self {
            let previous = std::env::var("CENSUS_EXPORT_GENERATION").ok();
            std::env::remove_var("CENSUS_EXPORT_GENERATION");
            Self { previous }
        }
    }

    impl Drop for ExportGenerationGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var("CENSUS_EXPORT_GENERATION", value),
                None => std::env::remove_var("CENSUS_EXPORT_GENERATION"),
            }
        }
    }

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
            report_export_key("core", &generation),
            report_export_key("core", &generation)
        );
        assert_eq!(report_export_key("core", &generation), "report:core:1");
        assert_eq!(
            bests_export_key("all", "2027", "50", &generation),
            bests_export_key("all", "2027", "50", &generation)
        );
        assert_eq!(
            consolidate_export_key(&generation),
            consolidate_export_key(&generation)
        );
        assert_eq!(consolidate_export_key(&generation), "consolidate:1");
        Ok(())
    }

    #[test]
    fn a_fresh_generation_produces_fresh_export_keys() -> TestResult {
        let previous = ExportGeneration::default_generation();
        let next = ExportGeneration::parse("2")?;
        assert_ne!(
            report_export_key("core", &previous),
            report_export_key("core", &next)
        );
        assert_eq!(report_export_key("core", &next), "report:core:2");
        assert_ne!(
            bests_export_key("core", "2027", "all", &previous),
            bests_export_key("core", "2027", "all", &next)
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
    fn the_environment_selector_drives_export_generation() -> TestResult {
        {
            let _guard = ExportGenerationGuard::clear();
            assert_eq!(export_generation()?.as_str(), DEFAULT_GENERATION);
        }
        {
            let _guard = ExportGenerationGuard::set("7");
            assert_eq!(export_generation()?.as_str(), "7");
            assert_eq!(
                report_export_key("core", &export_generation()?),
                "report:core:7"
            );
        }
        {
            let _guard = ExportGenerationGuard::set("bad/generation");
            assert!(export_generation().is_err());
        }
        Ok(())
    }
}
