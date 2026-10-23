use super::JurisdictionCensus;
use crate::census::profiles::Progress;
use crate::restate_services::wire::JurisdictionRequest;
use crate::restate_services::{job_error, CaptureExecution};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};

mod step;

const KEY: &str = "public_profile_collection_v1";
const MAX_STEPS: u16 = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub(crate) original: JurisdictionRequest,
    pub(crate) max_steps: u16,
}

impl JurisdictionCensus {
    pub(super) async fn collect_public_profiles(
        &self,
        ctx: ObjectContext<'_>,
        request: Request,
    ) -> Result<Progress, HandlerError> {
        if request.max_steps == 0 || request.max_steps > MAX_STEPS {
            return Err(TerminalError::new("profile max_steps must be in 1..=128").into());
        }
        self.require_bound_run(&request.original)?;
        let state = self.load_object(&ctx).await?;
        self.require_profile_collection_owner(&ctx, &request.original, &state)
            .await?;
        let mut progress = load_progress(&ctx, &state).await?;
        if progress.exhausted || progress.stopped || progress.awaiting_rosters {
            return Ok(progress);
        }
        let CaptureExecution::FreshQualification(writer) = &self.capture else {
            return Err(TerminalError::new("profile capture writer absent").into());
        };
        let fetcher = self
            .fetcher(
                &request.original.authorized_hosts,
                request.original.source_parallelism,
            )
            .await?;
        let cohort = self
            .store
            .run_manifest()
            .map_err(|error| job_error(error.into()))?
            .ok_or_else(|| TerminalError::new("profile bound manifest absent"))?
            .cohort;
        self.run_profile_batch(
            &ctx,
            step::Batch {
                request: &request.original,
                writer,
                fetcher: &fetcher,
                cohort,
                rosters_terminal: state
                    .rosters
                    .as_ref()
                    .is_some_and(|rosters| rosters.is_terminal()),
            },
            request.max_steps,
            &mut progress,
        )
        .await?;
        Ok(progress)
    }
}

async fn load_progress(
    ctx: &ObjectContext<'_>,
    state: &crate::restate_services::wire::JurisdictionState,
) -> Result<Progress, HandlerError> {
    let mut progress = ctx
        .get::<Json<Progress>>(KEY)
        .await?
        .map_or_else(Progress::default, |value| value.0);
    let witness = census_domain::model::serialized_digest(&state.rosters)
        .map_err(|error| TerminalError::new(error.to_string()))?;
    let witness = census_domain::model::AcquisitionDigest::parse(&witness)
        .map_err(|error| TerminalError::new(error.to_string()))?;
    progress
        .bind_rosters(witness)
        .map_err(|error| job_error(error.into()))?;
    ctx.set(KEY, Json(progress.clone()));
    Ok(progress)
}
