use restate_sdk::prelude::*;

use crate::restate_services::wire::{
    NationalRequest, NationalRequestBinding, PublicationContract, SchoolAddressJoinRequest,
};

pub(super) const KEY_ORIGINAL: &str = "national_request_original_managed_bundle_v2";
pub(super) const KEY_BOUND: &str = "national_request_managed_bundle_v2";

#[tracing::instrument(skip_all)]
pub(super) async fn prepare(
    ctx: &WorkflowContext<'_>,
    request: &mut NationalRequest,
) -> Result<(), HandlerError> {
    validate_publication_request(request)?;
    if request.publication_contract != PublicationContract::LegacySeparateV1 {
        ctx.set(KEY_ORIGINAL, Json(request.clone()));
    }
    super::redrive::bind_run(ctx, request).await?;
    if let Some(address) = request.school_address.as_mut() {
        let Json(digest) = ctx
            .service_client::<crate::restate_services::census::CensusClient>()
            .school_address_preflight(Json(address.clone()))
            .call()
            .await?;
        pin_address(address, digest, request.publication_contract)?;
    }
    if request.publication_contract != PublicationContract::LegacySeparateV1 {
        ctx.set(KEY_BOUND, Json(request.clone()));
    }
    Ok(())
}

fn validate_publication_request(request: &NationalRequest) -> Result<(), HandlerError> {
    let valid = match request.publication_contract {
        PublicationContract::LegacySeparateV1 | PublicationContract::ManagedBundleV2 => {
            matches!(
                &request.export_job,
                census_report::export::ExportJobRequest::LegacyV2
            )
        }
        PublicationContract::SelectedPublicationV1 => matches!(
            &request.export_job,
            census_report::export::ExportJobRequest::SelectedPublicationV1 { .. }
        ),
    };
    if valid {
        return Ok(());
    }
    Err(TerminalError::new(
        "national publication request contract and immutable profile selection disagree",
    )
    .into())
}

fn pin_address(
    address: &mut SchoolAddressJoinRequest,
    digest: String,
    contract: PublicationContract,
) -> Result<(), HandlerError> {
    if contract != PublicationContract::LegacySeparateV1
        && address
            .expected_digest
            .as_ref()
            .is_some_and(|expected| expected != &digest)
    {
        return Err(TerminalError::new(
            "bound national school-address directory digest changed; original request was preserved",
        ).into());
    }
    address.expected_digest = Some(digest);
    Ok(())
}

#[tracing::instrument(skip_all)]
pub(super) async fn read(
    ctx: &SharedWorkflowContext<'_>,
) -> Result<NationalRequestBinding, HandlerError> {
    if let Some(Json(request)) = ctx.get::<Json<NationalRequest>>(KEY_BOUND).await? {
        return Ok(NationalRequestBinding::Bound(Box::new(request)));
    }
    if ctx
        .get::<Json<NationalRequest>>(KEY_ORIGINAL)
        .await?
        .is_some()
    {
        return Ok(NationalRequestBinding::PendingPreflight);
    }
    Ok(NationalRequestBinding::UnboundLegacy)
}
