pub(crate) mod assembly;
pub(crate) mod team;

mod folding;
mod intake;
mod parsing;
mod reporting;
mod state;

use self::folding::{initial_phase, team_phase};
use self::reporting::{publish, publish_probe, terminal, validate_key};
use self::state::{profile_probe, state_from_probe};

use super::{
    acquisition::{ProfileAcquisition, ProfileJob, ProfileProbe},
    Runtime,
};
use crate::domain::identity::EvidenceDigest;
use anyhow::Result;
use restate_sdk::prelude::*;
use std::sync::Arc;

pub struct ProfileWorker {
    pub runtime: Arc<Runtime>,
}

#[restate_sdk::object(
    ingress_private = true,
    inactivity_timeout = "2h",
    journal_retention = "30 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 3, on_max_attempts = "pause")
)]
impl ProfileWorker {
    #[handler]
    pub async fn identify(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<ProfileJob>,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        let job = input.into_inner();
        validate_key(&ctx, &job)?;
        self.identify_probe(&ctx, &job).await
    }

    #[handler]
    pub async fn gather(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<ProfileJob>,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        let job = input.into_inner();
        validate_key(&ctx, &job)?;
        if let Some(result) = ctx.get::<Json<EvidenceDigest>>("result").await? {
            return Ok(result);
        }
        let artifact = self.build(&ctx, &job).await?;
        publish(&ctx, self.runtime.clone(), artifact).await
    }

    async fn identify_probe(
        &self,
        ctx: &ObjectContext<'_>,
        job: &ProfileJob,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        if let Some(probe) = ctx.get::<Json<EvidenceDigest>>("probe").await? {
            return Ok(probe);
        }
        let state = initial_phase(ctx, self.runtime.clone(), job).await?;
        publish_probe(
            ctx,
            self.runtime.clone(),
            profile_probe(job.athlete_id, state),
        )
        .await
    }

    async fn build(
        &self,
        ctx: &ObjectContext<'_>,
        job: &ProfileJob,
    ) -> Result<ProfileAcquisition, HandlerError> {
        let digest = self.identify_probe(ctx, job).await?;
        let state = self
            .runtime
            .load_json::<ProfileProbe>(&digest.0)
            .await
            .map(state_from_probe)
            .map_err(terminal)?;
        team_phase(ctx, self.runtime.clone(), job, state).await
    }
}
