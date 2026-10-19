use super::{Cli, Ingress, NationalArgs, NationalRequest, NonZeroU32, Result, Revision};
use census_domain::{model::SchoolYear, UsJurisdiction};
use census_service::restate_services::{
    HistoryWindow, PublicationContract, SchoolAddressJoinRequest,
};
#[tracing::instrument(skip_all)]
pub(super) async fn show_retained_request(
    ingestion: &Ingress,
    identity: &census_reconcile::identity::WorkflowIdentity,
) -> Result<()> {
    let request = super::resume::retained_request(ingestion, identity).await?;
    super::resume::validate_selected(&request)?;
    println!("existing invocation {} retains its bound publication request; submitted parameter changes were ignored", identity.as_str());
    println!("retained request\t{}", serde_json::to_string(&request)?);
    Ok(())
}

#[tracing::instrument(skip_all)]
pub(super) async fn national_request(
    cli: &Cli,
    args: &NationalArgs,
    ingestion: &Ingress,
    season: SchoolYear,
    revision: Revision,
    jurisdictions: &[UsJurisdiction],
    ordinal: Option<NonZeroU32>,
) -> Result<NationalRequest> {
    if let Some(ordinal) = ordinal {
        let request =
            super::resume::bound_request(ingestion, season, revision, jurisdictions, ordinal)
                .await?;
        println!(
            "resume\tpreserving the original selected publication request and execution budgets"
        );
        return Ok(request);
    }
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
    let export_job = crate::cli::publication_preflight::selected_request(
        args.selected_store_identity.as_deref(),
        args.selected_profile_id.as_deref(),
    )?;
    Ok(NationalRequest {
        season,
        revision,
        history,
        publication_contract: PublicationContract::SelectedPublicationV1,
        export_job,
        jurisdictions: jurisdictions.to_vec(),
        refresh: args.refresh,
        limit_per_state: args.limit_per_state,
        concurrency: args.concurrency,
        observed_on: None,
        authorized_hosts: cli.authorized_hosts.clone(),
        source_parallelism: cli.source_parallelism,
        school_address: school_address_request(args)?,
        pass_budget: args.pass_budget,
        pass_delay_seconds: args.pass_delay_seconds,
    })
}

fn school_address_request(args: &NationalArgs) -> Result<Option<SchoolAddressJoinRequest>> {
    if args.school_directory.is_none()
        && args.evidence_urls.is_empty()
        && args.evidence_dates.is_empty()
    {
        return Ok(None);
    }
    let overrides = census_service::school_address::Overrides::from_cli_pairs(
        &args.evidence_urls,
        &args.evidence_dates,
    )?;
    Ok(Some(SchoolAddressJoinRequest {
        generation: args.school_directory.clone(),
        overrides,
        expected_digest: None,
    }))
}
