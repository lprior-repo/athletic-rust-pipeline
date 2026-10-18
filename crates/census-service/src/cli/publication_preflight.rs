use anyhow::{bail, Context, Result};
use census_crawl::ingress::Ingress;
use restate_sdk::prelude::*;

use census_service::restate_services::{
    CensusIngressClient, PublicationCapabilities, PublicationContract,
};

#[tracing::instrument(skip_all)]
pub(super) async fn require_managed_publication(ingestion: &Ingress) -> Result<()> {
    require_publication_contract(ingestion, PublicationContract::ManagedBundleV2).await
}

pub(super) async fn require_publication_contract(
    ingestion: &Ingress,
    contract: PublicationContract,
) -> Result<()> {
    let Json(capabilities) = CensusIngressClient::from_client(ingestion.clone())
        .publication_capabilities()
        .call()
        .await
        .map_err(census_service::ingress::error)
        .context("publication compatibility preflight failed before submission")?
        .into_body()
        .map_err(census_service::ingress::error)
        .context("publication capability response is invalid; no invocation was submitted")?;
    require_contract(&capabilities, contract)
}

fn require_contract(
    capabilities: &PublicationCapabilities,
    contract: PublicationContract,
) -> Result<()> {
    if capabilities.publication_contracts.contains(&contract) {
        return Ok(());
    }
    bail!("endpoint does not advertise {contract:?}; no invocation was submitted")
}

#[cfg(test)]
mod tests;
