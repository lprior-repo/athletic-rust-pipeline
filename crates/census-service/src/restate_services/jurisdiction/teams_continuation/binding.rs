use super::{bounds::Budget, checked::digest, refusal};
use crate::restate_services::wire::{
    JurisdictionRequest, JurisdictionState, NationalRequest, NationalRequestBinding,
};
use crate::restate_services::{blocking, job_error, CaptureExecution, NationalCensusClient};
use census_domain::model::{AcquisitionDigest, CaptureCacheIntent, Revision, RunManifest};
use census_reconcile::identity::WorkflowIdentity;
use census_store::{CaptureWriterV1, Store, StoreError, StoreResult};
use restate_sdk::prelude::*;
use std::sync::Arc;

impl super::JurisdictionCensus {
    pub(super) async fn require_team_continuation_owner(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        state: &JurisdictionState,
        budget: &mut Budget,
    ) -> Result<(), HandlerError> {
        let CaptureExecution::FreshQualification(writer) = &self.capture else {
            return Err(refusal("current capture writer absent"));
        };
        let manifest = self
            .store
            .run_manifest()
            .map_err(|error| refusal(&error.to_string()))?
            .ok_or_else(|| refusal("bound run manifest absent"))?;
        let key = owner_key(&manifest);
        let Json(binding) = ctx
            .workflow_client::<NationalCensusClient>(key)
            .request()
            .call()
            .await?;
        let NationalRequestBinding::Bound(bound) = binding else {
            return Err(refusal("original bound national request absent"));
        };
        budget.value(&bound)?;
        super::super::team_source::continuation_scope::require_national_scope(&manifest, &bound)?;
        if digest(&bound.for_jurisdiction(request.jurisdiction))? != digest(request)? {
            return Err(refusal(
                "jurisdiction differs from full bound national request",
            ));
        }
        let plan = state
            .plan
            .as_ref()
            .ok_or_else(|| refusal("retained plan absent"))?;
        let fingerprint = AcquisitionDigest::parse(&plan.fingerprint)
            .map_err(|error| refusal(&error.to_string()))?;
        let store = Arc::clone(&self.store);
        let writer = Arc::clone(writer);
        let region = Arc::clone(&self.region);
        let request = request.clone();
        ctx.run(move || async move {
            let permit = self.jobs.permit_control().await?;
            blocking(region, move || {
                let _permit = permit;
                require_owner(&store, &writer, &manifest, &bound, &request, &fingerprint)
            })
            .await
            .map_err(job_error)
        })
        .name("teams:continuation:owner-binding")
        .retry_policy(RunRetryPolicy::new().max_attempts(1))
        .await
        .map_err(Into::into)
    }
}

fn require_owner(
    store: &Store,
    writer: &CaptureWriterV1,
    manifest: &RunManifest,
    bound: &NationalRequest,
    request: &JurisdictionRequest,
    fingerprint: &AcquisitionDigest,
) -> StoreResult<()> {
    let owner = writer.public_resource_owner(store, request.jurisdiction)?;
    let scope = owner.scope();
    let key =
        WorkflowIdentity::jurisdiction(request.jurisdiction, request.season, request.revision);
    let manifest_digest = acquisition_digest(manifest)?;
    let intent = if request.refresh {
        CaptureCacheIntent::AcquireNew
    } else {
        CaptureCacheIntent::ReuseEligible
    };
    if scope.owner_key() != key.as_str()
        || scope.jurisdiction() != request.jurisdiction
        || scope.run() != manifest.run
        || scope.run_manifest_digest() != &manifest_digest
        || scope.history() != request.history
        || scope.cache_intent() != intent
    {
        return Err(StoreError::Refused {
            detail: "ordinary teams continuation owner scope".into(),
        });
    }
    owner.require_service_binding(
        store,
        bound.export_job.require_selected_publication(),
        &owner_key(manifest),
        &acquisition_digest(bound)?,
        &acquisition_digest(request)?,
        fingerprint,
    )
}

fn acquisition_digest(value: &(impl serde::Serialize + ?Sized)) -> StoreResult<AcquisitionDigest> {
    AcquisitionDigest::parse(&census_domain::model::serialized_digest(value)?)
        .map_err(|error| census_store::CaptureError::Domain(error).into())
}

fn owner_key(manifest: &RunManifest) -> String {
    WorkflowIdentity::national(
        manifest.run.season(),
        Revision(manifest.run.revision()),
        &manifest.jurisdictions,
    )
    .to_string()
}
