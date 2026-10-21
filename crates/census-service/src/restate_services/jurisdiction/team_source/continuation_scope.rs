use super::TeamsSource;
use crate::restate_services::wire::{NationalRequest, NationalRequestBinding, TeamsSourceRequest};
use crate::restate_services::NationalCensusClient;
use census_domain::model::{serialized_digest, Revision, RunManifest};
use census_reconcile::identity::{admitted_scope, WorkflowIdentity};
use restate_sdk::prelude::*;

impl TeamsSource {
    pub(super) async fn require_continuation_scope(
        &self,
        ctx: &ObjectContext<'_>,
        original: &TeamsSourceRequest,
    ) -> Result<(), HandlerError> {
        let manifest = self
            .owner
            .store
            .run_manifest()
            .map_err(|error| refusal(&error.to_string()))?
            .ok_or_else(|| refusal("origin continuation requires a bound run manifest"))?;
        let workflow = owner_key(&manifest);
        let Json(binding) = ctx
            .workflow_client::<NationalCensusClient>(workflow)
            .request()
            .call()
            .await?;
        let NationalRequestBinding::Bound(bound) = binding else {
            return Err(refusal(
                "origin continuation requires the original bound national request",
            ));
        };
        require_scope(&manifest, &bound, original)
    }
}

fn owner_key(manifest: &RunManifest) -> String {
    WorkflowIdentity::national(
        manifest.run.season(),
        Revision(manifest.run.revision()),
        &manifest.jurisdictions,
    )
    .to_string()
}

fn require_scope(
    manifest: &RunManifest,
    bound: &NationalRequest,
    original: &TeamsSourceRequest,
) -> Result<(), HandlerError> {
    let scope = admitted_scope(&bound.jurisdictions);
    let same_scope = scope.len() == manifest.jurisdictions.len()
        && scope
            .iter()
            .all(|place| manifest.jurisdictions.contains(place))
        && scope
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == scope.len();
    if !same_scope
        || !scope.contains(&original.jurisdiction.jurisdiction)
        || bound.season != manifest.run.season()
        || bound.revision.get() != manifest.run.revision()
        || WorkflowIdentity::national(bound.season, bound.revision, &bound.jurisdictions).as_str()
            != owner_key(manifest)
        || serialized_digest(&bound.for_jurisdiction(original.jurisdiction.jurisdiction))
            .map_err(|error| refusal(&error.to_string()))?
            != serialized_digest(&original.jurisdiction)
                .map_err(|error| refusal(&error.to_string()))?
    {
        return Err(refusal(
            "origin continuation request differs from its bound national request",
        ));
    }
    Ok(())
}

fn refusal(message: &str) -> HandlerError {
    TerminalError::new(message).into()
}

#[cfg(test)]
mod tests;
