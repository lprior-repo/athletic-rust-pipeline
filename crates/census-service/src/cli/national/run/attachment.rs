use anyhow::{Context, Result};
use census_crawl::ingress::{CurrentRoute, Ingress};
use census_domain::national_census::NationalReport;
use census_reconcile::identity::WorkflowIdentity;
use census_service::ingress;
use restate_sdk::{ingress::InvocationHandle, prelude::Json};

#[tracing::instrument(skip_all)]
pub(crate) async fn attach_existing(
    origin: &str,
    ingestion: &Ingress,
    identity: &WorkflowIdentity,
) -> Result<InvocationHandle<CurrentRoute, Json<NationalReport>>> {
    let invocation_id = ingress::lookup_workflow(origin, "NationalCensus", identity.as_str())
        .await
        .context("national attachment could not resolve the existing workflow")?;
    let handle = ingestion.invocation_handle(invocation_id);
    let attachment_context = || {
        format!(
            "no readable invocation exists under {} (invocation {})",
            identity.as_str(),
            handle.invocation_id()
        )
    };
    let output = handle
        .output()
        .await
        .map_err(super::super::output::read_error)
        .with_context(attachment_context)?;
    super::super::output::validate(
        output.headers(),
        output.body(),
        handle.invocation_id(),
        identity,
    )
    .with_context(attachment_context)?;
    Ok(handle)
}
