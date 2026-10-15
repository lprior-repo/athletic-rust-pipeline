use anyhow::{bail, Context, Result};
use census_crawl::ingress::Ingress;
use restate_sdk::prelude::*;

use crate::restate_services::{CensusIngressClient, PublicationCapabilities, PublicationContract};

#[tracing::instrument(skip_all)]
pub async fn require_managed_publication(ingestion: &Ingress) -> Result<()> {
    let Json(capabilities) = CensusIngressClient::from_client(ingestion.clone())
        .publication_capabilities()
        .call()
        .await
        .map_err(super::error)
        .context("publication compatibility preflight failed before submission")?
        .into_body()
        .map_err(super::error)
        .context("publication capability response is invalid; no invocation was submitted")?;
    require_managed(&capabilities)
}

fn require_managed(capabilities: &PublicationCapabilities) -> Result<()> {
    if !capabilities
        .publication_contracts
        .contains(&PublicationContract::ManagedBundleV2)
    {
        bail!("endpoint does not advertise managed_bundle_v2; no invocation was submitted");
    }
    Ok(())
}

#[cfg(test)]
mod tests;
