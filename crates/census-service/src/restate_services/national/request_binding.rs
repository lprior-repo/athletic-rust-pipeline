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
    let original = request.clone();
    ctx.service_client::<crate::restate_services::census::CensusClient>()
        .workbook_preflight(Json(super::publication::workbook_request(request)))
        .call()
        .await?;
    if let Some(address) = request.school_address.as_mut() {
        let Json(digest) = ctx
            .service_client::<crate::restate_services::census::CensusClient>()
            .school_address_preflight(Json(address.clone()))
            .call()
            .await?;
        pin_address(address, digest)?;
    }
    ctx.set(KEY_ORIGINAL, Json(original));
    super::redrive::bind_run(ctx, request).await?;
    ctx.set(KEY_BOUND, Json(request.clone()));
    Ok(())
}

fn validate_publication_request(request: &NationalRequest) -> Result<(), HandlerError> {
    if request.publication_contract != PublicationContract::SelectedPublicationV1 {
        return Err(TerminalError::new_with_code(
            400,
            "current national publication requires the selected publication contract",
        )
        .into());
    }
    request
        .export_job
        .require_selected_publication()
        .map_err(|error| TerminalError::new_with_code(400, error.to_string()))?;
    super::publication::generation(request)?;
    Ok(())
}

fn pin_address(address: &mut SchoolAddressJoinRequest, digest: String) -> Result<(), HandlerError> {
    if address
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
