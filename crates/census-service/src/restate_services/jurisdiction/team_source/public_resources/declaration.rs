use crate::restate_services::wire::{JurisdictionState, NationalRequest};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_reconcile::identity::WorkflowIdentity;
use census_store::{PublicResourceDeclarationBindingV1, PublicResourceOwnerBindingV1};
use census_store::{StoreError, StoreResult};

pub(super) struct JurisdictionFacts {
    pub(super) site: UsJurisdiction,
    pub(super) state: JurisdictionState,
    pub(super) directory_request: RequestRepresentationV1,
    pub(super) robots_request: RequestRepresentationV1,
}

pub(super) fn build(
    manifest: &RunManifest,
    capture_digest: &AcquisitionDigest,
    bound: &NationalRequest,
    facts: &[JurisdictionFacts],
) -> StoreResult<PublicResourceDeclarationBindingV1> {
    let scope = super::super::continuation_scope::require_national_scope(manifest, bound)
        .map_err(|_| refused("public resource bound national scope"))?;
    let store = StoreIdentity::parse(&manifest.store_identity)
        .map_err(|error| refused(&error.to_string()))?;
    let selection = bound.export_job.require_selected_publication();
    selection
        .ensure_store(&store)
        .map_err(|error| refused(&error.to_string()))?;
    if manifest.cohort != GradYear::CO2027 || scope.is_empty() || facts.len() != scope.len() {
        return Err(refused("public resource declaration owner inventory"));
    }
    let mut owners = Vec::new();
    owners
        .try_reserve_exact(facts.len())
        .map_err(|_| refused("public resource owner allocation"))?;
    for fact in facts {
        if !scope.contains(&fact.site)
            || owners
                .iter()
                .any(|owner: &PublicResourceOwnerBindingV1| owner.site == fact.site)
        {
            return Err(refused("public resource declaration owner inventory"));
        }
        owners.push(owner(bound, fact)?);
    }
    owners.sort_unstable_by_key(|owner| owner.site);
    Ok(PublicResourceDeclarationBindingV1 {
        version: 1,
        store,
        run: manifest.run,
        capture_declaration_digest: capture_digest.clone(),
        run_manifest_digest: digest(manifest)?,
        selection: selection.clone(),
        national_owner_key: WorkflowIdentity::national(
            bound.season,
            bound.revision,
            &bound.jurisdictions,
        )
        .to_string(),
        national_request_digest: digest(bound)?,
        owners,
    })
}

fn owner(
    bound: &NationalRequest,
    facts: &JurisdictionFacts,
) -> StoreResult<PublicResourceOwnerBindingV1> {
    let request = bound.for_jurisdiction(facts.site);
    let key = WorkflowIdentity::jurisdiction(facts.site, bound.season, bound.revision);
    let plan = facts
        .state
        .plan
        .as_ref()
        .ok_or_else(|| refused("public resource retained source plan absent"))?;
    if facts.state.identity != key.as_str()
        || facts.state.history_window != Some(request.history)
        || plan
            .sweepable
            .iter()
            .filter(|slug| slug.as_str() == "milesplit")
            .count()
            != 1
        || plan.refused.iter().any(|source| source.slug == "milesplit")
        || plan
            .satisfied
            .iter()
            .any(|source| source.slug == "milesplit")
    {
        return Err(refused("public resource retained jurisdiction binding"));
    }
    validate_requests(facts)?;
    Ok(PublicResourceOwnerBindingV1 {
        owner_key: key.to_string(),
        site: facts.site,
        jurisdiction_request_digest: digest(&request)?,
        source_plan_fingerprint: AcquisitionDigest::parse(&plan.fingerprint).map_err(domain)?,
        history: request.history,
        cache_intent: if request.refresh {
            CaptureCacheIntent::AcquireNew
        } else {
            CaptureCacheIntent::ReuseEligible
        },
        directory_request: facts.directory_request.clone(),
        robots_request: facts.robots_request.clone(),
    })
}

fn validate_requests(facts: &JurisdictionFacts) -> StoreResult<()> {
    let site = census_crawl::milesplit::Site::for_jurisdiction(facts.site);
    let origin = format!("https://{}", site.host());
    if !facts.directory_request.observed_http_required()
        || !facts.robots_request.observed_http_required()
        || facts.directory_request.original_url() != site.teams_url()
        || facts.robots_request.original_url() != format!("{origin}/robots.txt")
    {
        return Err(refused("public resource bound root request"));
    }
    PublicSourceResourceV1::milesplit_team_directory()
        .validate_target(facts.site, facts.directory_request.original_url())
        .map_err(domain)?;
    PublicSourceResourceV1::robots(&origin)
        .and_then(|resource| {
            resource.validate_target(facts.site, facts.robots_request.original_url())
        })
        .map_err(domain)
}

fn digest(value: &(impl serde::Serialize + ?Sized)) -> StoreResult<AcquisitionDigest> {
    AcquisitionDigest::parse(&serialized_digest(value)?).map_err(domain)
}

fn domain(error: CaptureDomainError) -> StoreError {
    census_store::CaptureError::Domain(error).into()
}

fn refused(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests;
