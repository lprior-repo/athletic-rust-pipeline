use anyhow::Result;
use census_crawl::ingress::Ingress;
use census_domain::UsJurisdiction;
use census_reconcile::identity::WorkflowIdentity;
use census_service::restate_services::{
    ContactStatus, ContactSummary, NationalCensusIngressClient,
};
use restate_sdk::prelude::*;
use std::time::Duration;

const UNKNOWN: &str = "publication unconfirmed: contact measurement unavailable; a final national report does not certify a workbook or seal";
const UNVERIFIED: &str = "publication unconfirmed: contact measurement alone does not certify a workbook or seal; verify the manifested workbook and the run's seal";

#[tracing::instrument(skip_all)]
pub(super) async fn show(
    ingestion: &Ingress,
    identity: &WorkflowIdentity,
    jurisdictions: &[UsJurisdiction],
) {
    let budget = Duration::from_secs(5);
    let notice = match tokio::time::timeout(budget, read(ingestion, identity)).await {
        Ok(Ok(Some(contacts))) => describe(&contacts, jurisdictions),
        Ok(Ok(None) | Err(_)) | Err(_) => UNKNOWN.to_owned(),
    };
    eprintln!("{}: {notice}", identity.as_str());
}

#[tracing::instrument(skip_all)]
async fn read(
    ingestion: &Ingress,
    identity: &WorkflowIdentity,
) -> Result<Option<Vec<ContactSummary>>> {
    let Json(contacts) =
        NationalCensusIngressClient::from_client(ingestion.clone(), identity.as_str())
            .contact_report()
            .call()
            .await
            .map_err(census_service::ingress::error)?
            .into_body()
            .map_err(census_service::ingress::error)?;
    Ok(contacts)
}

fn describe(contacts: &[ContactSummary], jurisdictions: &[UsJurisdiction]) -> String {
    if contacts.len() > UsJurisdiction::CENSUS_SCOPE.len()
        || jurisdictions.len() > UsJurisdiction::CENSUS_SCOPE.len()
    {
        return UNKNOWN.to_owned();
    }
    if contacts.len() != jurisdictions.len()
        || jurisdictions.iter().any(|jurisdiction| {
            contacts
                .iter()
                .filter(|contact| contact.jurisdiction == *jurisdiction)
                .count()
                != 1
        })
    {
        return "publication unconfirmed: contact measurement does not cover the exact run scope; a final report does not certify publication".to_owned();
    }
    let unfinished: Vec<_> = contacts
        .iter()
        .filter(|contact| !matches!(contact.status, ContactStatus::Complete))
        .map(|contact| {
            format!(
                "{} contacts {:?}",
                contact.jurisdiction.code(),
                contact.status
            )
        })
        .collect();
    if unfinished.is_empty() {
        UNVERIFIED.to_owned()
    } else {
        format!("publication incomplete: {}; a final national report does not certify a workbook or seal", unfinished.join(", "))
    }
}

#[cfg(test)]
mod tests;
