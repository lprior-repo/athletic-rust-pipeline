use anyhow::{bail, Context, Result};
use census_crawl::ingress::Ingress;
use census_domain::{
    model::{ProfileId, Revision, SchoolYear, StoreIdentity},
    UsJurisdiction,
};
use census_reconcile::identity::WorkflowIdentity;
use census_service::restate_services::{
    ExportGeneration, HistoryWindow, NationalCensusIngressClient, NationalRequest,
    NationalRequestBinding, PublicationContract,
};
use futures::{StreamExt, TryStreamExt};
use restate_sdk::prelude::*;
use std::num::NonZeroU32;

#[tracing::instrument(skip_all)]
pub(in crate::cli::national::run) async fn bound_request(
    ingestion: &Ingress,
    season: SchoolYear,
    revision: Revision,
    jurisdictions: &[UsJurisdiction],
    ordinal: NonZeroU32,
) -> Result<NationalRequest> {
    let bindings = futures::stream::iter(candidates(season, revision, jurisdictions, ordinal))
        .then(|key| lookup(ingestion, key))
        .try_filter_map(|binding| futures::future::ready(resumable(binding)));
    futures::pin_mut!(bindings);
    let request = bindings.try_next().await?.ok_or_else(|| anyhow::anyhow!(
        "national resume compatibility blocked: selected, base and prior workflows have no bound selected publication request; legacy or unbound work requires its compatible client; no invocation was submitted"
    ))?;
    validate_identity(&request, season, revision, jurisdictions)?;
    let history = super::history(ingestion, season, revision, jurisdictions).await?;
    validate_history(&request, history)?;
    Ok(*request)
}

fn resumable(binding: NationalRequestBinding) -> Result<Option<Box<NationalRequest>>> {
    match binding {
        NationalRequestBinding::Bound(request) => Ok(Some(request)),
        NationalRequestBinding::UnboundLegacy => Ok(None),
        NationalRequestBinding::PendingPreflight => bail!(
            "national resume compatibility blocked: original school-address preflight is incomplete; resume the original invocation before submitting another execution"
        ),
    }
}

#[tracing::instrument(skip_all)]
pub(in crate::cli::national::run) async fn retained_request(
    ingestion: &Ingress,
    identity: &WorkflowIdentity,
) -> Result<NationalRequest> {
    let request = resumable(lookup(ingestion, identity.clone()).await?)?
        .ok_or_else(|| anyhow::anyhow!(
            "managed publication attachment compatibility blocked: existing invocation has no bound managed publication request; observe legacy work with its compatible client"
        ))?;
    validate_execution_identity(&request, identity)?;
    Ok(*request)
}

#[tracing::instrument(skip_all)]
async fn lookup(ingestion: &Ingress, key: WorkflowIdentity) -> Result<NationalRequestBinding> {
    let Json(binding) = NationalCensusIngressClient::from_client(ingestion.clone(), key.as_str())
        .request()
        .call()
        .await
        .map_err(census_service::ingress::error)
        .context("national resume compatibility blocked: original request lookup failed")?
        .into_body()
        .map_err(census_service::ingress::error)
        .context("national resume compatibility blocked: invalid request binding")?;
    Ok(binding)
}

fn candidates(
    season: SchoolYear,
    revision: Revision,
    jurisdictions: &[UsJurisdiction],
    ordinal: NonZeroU32,
) -> Vec<WorkflowIdentity> {
    let mut keys = vec![
        super::identity(season, revision, jurisdictions, Some(ordinal)),
        super::identity(season, revision, jurisdictions, None),
    ];
    if let Some(prior) = ordinal.get().checked_sub(1).and_then(NonZeroU32::new) {
        keys.push(super::identity(
            season,
            revision,
            jurisdictions,
            Some(prior),
        ));
    }
    keys
}

fn validate_identity(
    request: &NationalRequest,
    season: SchoolYear,
    revision: Revision,
    jurisdictions: &[UsJurisdiction],
) -> Result<()> {
    validate_selected(request)?;
    validate_execution_identity(
        request,
        &WorkflowIdentity::national(season, revision, jurisdictions),
    )
}

pub(in crate::cli::national::run) fn validate_selected(request: &NationalRequest) -> Result<()> {
    if !matches!(
        request.publication_contract,
        PublicationContract::SelectedPublicationV1 | PublicationContract::SelectedPublicationV2
    ) {
        bail!("national resume compatibility blocked: bound request is not a selected publication contract");
    }
    if !matches!(
        request.export_job,
        census_report::export::ExportJobRequest::SelectedPublicationV1 { .. }
    ) {
        bail!("national resume compatibility blocked: bound export request is not a selected publication request");
    }
    if request.generation.is_none() {
        bail!("national resume compatibility blocked: bound request has no publication generation");
    }
    Ok(())
}

fn validate_execution_identity(
    request: &NationalRequest,
    identity: &WorkflowIdentity,
) -> Result<()> {
    validate_managed(request)?;
    let expected = identity
        .as_str()
        .rsplit_once(":resume:")
        .map_or(identity.as_str(), |(base, _)| base);
    let actual =
        WorkflowIdentity::national(request.season, request.revision, &request.jurisdictions);
    if actual.as_str() != expected {
        bail!(
            "bound national request differs from the requested season, revision or jurisdictions"
        );
    }
    Ok(())
}

fn validate_managed(request: &NationalRequest) -> Result<()> {
    if request.publication_contract == PublicationContract::LegacySeparateV1 {
        bail!("national resume compatibility blocked: bound request is not a managed publication contract");
    }
    if request
        .school_address
        .as_ref()
        .is_some_and(|address| address.expected_digest.is_none())
    {
        bail!("national resume compatibility blocked: bound school-address request has no pinned directory digest");
    }
    Ok(())
}

fn validate_history(request: &NationalRequest, history: HistoryWindow) -> Result<()> {
    if request.history != history {
        bail!("bound national request history disagrees with retained jurisdiction history");
    }
    Ok(())
}

pub(in crate::cli::national::run) fn refuse_on_caller_authority(
    bound: &NationalRequest,
    caller_store: Option<&str>,
    caller_profile: Option<&str>,
    caller_generation: Option<&str>,
) -> Result<()> {
    match (caller_store, caller_profile) {
        (None, None) => {}
        (Some(store), Some(profile)) => {
            let store_id = StoreIdentity::parse(store)
                .context("national resume compatibility blocked: invalid caller store identity")?;
            let profile_id = ProfileId::parse(profile)
                .context("national resume compatibility blocked: invalid caller profile id")?;
            let selection = match &bound.export_job {
                census_report::export::ExportJobRequest::SelectedPublicationV1 { selection } => selection,
                _ => bail!("national resume compatibility blocked: bound export request is not a selected publication request"),
            };
            if store_id.as_str() != selection.store_identity().as_str()
                || profile_id.as_str() != selection.profile_id().as_str()
            {
                bail!("national authority compatibility blocked: caller store/profile selection differs from bound request; refusing resume or attachment");
            }
        }
        (Some(_), None) | (None, Some(_)) => {
            bail!("national resume compatibility blocked: store identity and profile id must be supplied together");
        }
    }
    if let Some(generation) = caller_generation {
        let caller_gen = ExportGeneration::parse(generation)
            .context("national resume compatibility blocked: invalid caller generation")?;
        match bound.generation.as_ref() {
            None => bail!("national resume compatibility blocked: bound request has no publication generation"),
            Some(bound_gen) if bound_gen.as_str() != caller_gen.as_str() => {
                bail!("national authority compatibility blocked: caller generation differs from bound request; refusing resume or attachment");
            }
            Some(_) => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
