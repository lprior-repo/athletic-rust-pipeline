use super::{operation, TeamsSource};
use crate::restate_services::wire::{NationalRequest, TeamsSourceRequest};
use crate::restate_services::{blocking, job_error, CaptureExecution, JurisdictionCensusClient};
use census_crawl::net::{FetchOptions, Fetcher};
use census_reconcile::identity::{admitted_scope, WorkflowIdentity};
use census_store::{Application, CaptureWriterV1, Store};
use restate_sdk::prelude::*;
use std::sync::Arc;

mod declaration;

impl TeamsSource {
    pub(super) fn public_resource_writer(&self) -> Result<&Arc<CaptureWriterV1>, HandlerError> {
        match &self.owner.capture {
            CaptureExecution::FreshQualification(writer) => Ok(writer),
            _ => Err(
                TerminalError::new("public resources require current capture qualification").into(),
            ),
        }
    }

    pub(super) async fn admit_public_resource_roots(
        &self,
        ctx: &ObjectContext<'_>,
        original: &TeamsSourceRequest,
    ) -> Result<Json<Application>, HandlerError> {
        if original.source != "milesplit"
            || operation::phase(ctx.key(), original).map_err(job_error)? != operation::Phase::Teams
        {
            return Err(TerminalError::new(
                "public resource admission differs from its source key",
            )
            .into());
        }
        let writer = self.public_resource_writer()?;
        let bound = self.bound_continuation_scope(ctx, original).await?;
        let fetcher = self
            .owner
            .fetcher(&bound.authorized_hosts, bound.source_parallelism)
            .await?;
        let facts = read_facts(ctx, &bound, &fetcher).await?;
        let store = Arc::clone(&self.owner.store);
        let writer = Arc::clone(writer);
        let region = Arc::clone(self.jobs.region());
        Ok(ctx
            .run(move || async move {
                let permit = self.jobs.permit_control().await?;
                blocking(region, move || {
                    let _permit = permit;
                    declare(&store, &writer, &bound, &facts)
                })
                .await
                .map(Json)
                .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?)
    }
}

async fn read_facts(
    ctx: &ObjectContext<'_>,
    bound: &NationalRequest,
    fetcher: &Fetcher,
) -> Result<Vec<declaration::JurisdictionFacts>, HandlerError> {
    let mut sites = admitted_scope(&bound.jurisdictions);
    sites.sort_unstable();
    let mut facts = Vec::new();
    facts
        .try_reserve_exact(sites.len())
        .map_err(|_| TerminalError::new("public resource owner allocation"))?;
    for site in sites {
        let key = WorkflowIdentity::jurisdiction(site, bound.season, bound.revision);
        let Json(state) = ctx
            .object_client::<JurisdictionCensusClient>(key.to_string())
            .state()
            .call()
            .await?;
        let root = census_crawl::milesplit::Site::for_jurisdiction(site);
        let options = FetchOptions {
            refresh: bound.refresh,
            allow_not_found: true,
            ..FetchOptions::default()
        };
        let directory_request = fetcher
            .public_resource_request(&root.teams_url(), &options)
            .map_err(|error| TerminalError::new(error.to_string()))?;
        let robots_request = fetcher
            .public_resource_request(&format!("https://{}/robots.txt", root.host()), &options)
            .map_err(|error| TerminalError::new(error.to_string()))?;
        facts.push(declaration::JurisdictionFacts {
            site,
            state,
            directory_request,
            robots_request,
        });
    }
    Ok(facts)
}

fn declare(
    store: &Store,
    writer: &CaptureWriterV1,
    bound: &NationalRequest,
    facts: &[declaration::JurisdictionFacts],
) -> Result<Application, crate::restate_services::JobError> {
    let manifest = store
        .run_manifest()?
        .ok_or_else(|| census_store::StoreError::Refused {
            detail: "public resources require the original run manifest".to_string(),
        })?;
    let capture_digest = writer.public_resource_capture_digest(store)?;
    let binding = declaration::build(&manifest, &capture_digest, bound, facts)?;
    Ok(store.declare_public_resource_capture(writer, binding, chrono::Utc::now())?)
}
