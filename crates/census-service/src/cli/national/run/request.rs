use super::{Cli, Ingress, NationalArgs, NationalRequest, Result, Revision};
use census_domain::{model::SchoolYear, UsJurisdiction};
use census_service::restate_services::{
    HistoryWindow, PublicationContract, SchoolAddressJoinRequest,
};
pub(super) async fn show_retained_request(
    ingestion: &Ingress,
    identity: &census_reconcile::identity::WorkflowIdentity,
    caller_store: Option<&str>,
    caller_profile: Option<&str>,
    caller_generation: Option<&str>,
) -> Result<()> {
    let request = super::resume::retained_request(ingestion, identity).await?;
    super::resume::validate_selected(&request)?;
    super::resume::refuse_on_caller_authority(
        &request,
        caller_store,
        caller_profile,
        caller_generation,
    )?;
    println!("existing invocation {} retains its original selected publication request and execution budgets", identity.as_str());
    println!("retained request\t{}", serde_json::to_string(&request)?);
    Ok(())
}

#[tracing::instrument(skip_all)]
pub(super) async fn national_request(
    cli: &Cli,
    args: &NationalArgs,
    season: SchoolYear,
    revision: Revision,
    jurisdictions: &[UsJurisdiction],
) -> Result<NationalRequest> {
    let history = HistoryWindow::cohort(&chrono::Utc::now().date_naive().to_string())
        .map_err(anyhow::Error::msg)?;
    fresh_request(cli, args, season, revision, jurisdictions, history)
}

fn fresh_request(
    cli: &Cli,
    args: &NationalArgs,
    season: SchoolYear,
    revision: Revision,
    jurisdictions: &[UsJurisdiction],
    history: HistoryWindow,
) -> Result<NationalRequest> {
    let generation =
        census_service::restate_services::ExportGeneration::resolve(args.generation.as_deref())?;
    let export_job = crate::cli::publication_preflight::selected_request(
        args.selected_store_identity.as_deref(),
        args.selected_profile_id.as_deref(),
    )?;
    Ok(NationalRequest {
        season,
        revision,
        history,
        publication_contract: PublicationContract::SelectedPublicationV2,
        export_job,
        jurisdictions: jurisdictions.to_vec(),
        refresh: args.refresh,
        limit_per_state: args.limit_per_state,
        concurrency: args.concurrency,
        observed_on: None,
        authorized_hosts: cli.authorized_hosts.clone(),
        source_parallelism: cli.source_parallelism,
        school_address: school_address_request(args)?,
        pass_budget: census_service::restate_services::default_pass_budget(),
        pass_delay_seconds: census_service::restate_services::default_pass_delay_seconds(),
        generation: Some(generation),
    })
}

fn school_address_request(args: &NationalArgs) -> Result<Option<SchoolAddressJoinRequest>> {
    let Some(generation) = args.school_directory.clone() else {
        return Ok(None);
    };
    Ok(Some(SchoolAddressJoinRequest {
        generation: Some(generation),
        overrides: census_service::school_address::Overrides::default(),
        expected_digest: None,
    }))
}
