use anyhow::{bail, Context, Result};
use census_crawl::ingress::Ingress;
use census_domain::model::{ProfileId, ProfileSelectionV1, StoreIdentity};
use census_report::export::ExportJobRequest;
use census_service::restate_services::{
    CensusIngressClient, PublicationCapabilities, PublicationContract,
};
use restate_sdk::prelude::*;

pub(super) fn selected_request(
    store_identity: Option<&str>,
    profile_id: Option<&str>,
) -> Result<ExportJobRequest> {
    let (Some(store_identity), Some(profile_id)) = (store_identity, profile_id) else {
        bail!("--selected-store-identity and --selected-profile-id must be supplied together");
    };
    Ok(ExportJobRequest::SelectedPublicationV1 {
        selection: ProfileSelectionV1::new(
            StoreIdentity::parse(store_identity).map_err(anyhow::Error::msg)?,
            ProfileId::parse(profile_id).map_err(anyhow::Error::msg)?,
        ),
    })
}

pub(super) fn open_selected_store(root: &std::path::Path) -> Result<census_store::Store> {
    let (store, writer) = census_store::Store::open_public_identity_qualification_v3(root)
        .context("opening existing selected-publication authority without enrollment")?
        .into_parts();
    drop(writer);
    Ok(store)
}

pub(super) async fn require_publication_contract(
    ingestion: &Ingress,
    contract: PublicationContract,
) -> Result<()> {
    let capabilities = capabilities(ingestion).await?;
    require_contract(&capabilities, contract)
}

pub(super) async fn require_national_publication(ingestion: &Ingress) -> Result<()> {
    let capabilities = capabilities(ingestion).await?;
    require_contract(&capabilities, PublicationContract::SelectedPublicationV1)?;
    if !capabilities.supports_national_generation() {
        bail!("endpoint does not advertise stable national generation support; no invocation was submitted");
    }
    Ok(())
}

async fn capabilities(ingestion: &Ingress) -> Result<PublicationCapabilities> {
    let Json(capabilities) = CensusIngressClient::from_client(ingestion.clone())
        .publication_capabilities()
        .call()
        .await
        .map_err(census_service::ingress::error)
        .context("publication compatibility preflight failed before submission")?
        .into_body()
        .map_err(census_service::ingress::error)
        .context("publication capability response is invalid; no invocation was submitted")?;
    Ok(capabilities)
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
