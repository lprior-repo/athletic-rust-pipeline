use std::sync::Arc;
use std::time::Duration;

use restate_sdk::prelude::*;

use crate::census::seal::{self, census_cohort};
use census_domain::model::{CensusRun, GradYear, RunManifest, SchoolYear};
use census_report::export::store_identity;
use census_store::clock::Clock;
use census_store::Store;

use super::publish::Jobs;
use super::wire::{
    BindRunReply, BindRunRequest, OpenWorkReply, OpenWorkRequest, SealReply, SealRequest,
    StatusReply, TableCount,
};
use super::{job_error, JobError};

mod preflight;
mod requests;
mod status;

#[derive(Clone)]
pub struct Census {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
    jobs: Jobs,
}

impl Census {
    pub fn new(store: Arc<Store>, clock: Arc<dyn Clock>, jobs: Jobs) -> Self {
        Self { store, clock, jobs }
    }

    async fn store_evidence(
        &self,
        request: &OpenWorkRequest,
    ) -> Result<super::open_work::StoreEvidence, HandlerError> {
        let season = SchoolYear::new(request.season)
            .ok_or_else(|| TerminalError::new("invalid open-work season"))?;
        let revision = census_reconcile::identity::Revision(request.revision);
        let store = Arc::clone(&self.store);
        let region = Arc::clone(self.jobs.region());
        let permit = self.jobs.permit_control().await?;
        super::blocking(region, move || {
            let _permit = permit;
            super::open_work::inspect_store(&store, season, revision)
        })
        .await
        .map_err(job_error)
    }

    async fn seal_reply(
        &self,
        ctx: &Context<'_>,
        request: &SealRequest,
        refusal: RefusalPolicy,
    ) -> Result<Json<SealReply>, HandlerError> {
        if let Err(error) = census_cohort(request.grad_year) {
            let message = match error {
                seal::SealWorkflowError::ForeignCohort { requested } => format!(
                    "the Class-of-2027 cohort is the only one that may be sealed; {requested} is not the census seal cohort"
                ),
                other => other.to_string(),
            };
            return Err(TerminalError::new(message).into());
        }
        let season = SchoolYear::new(request.season).ok_or_else(|| {
            TerminalError::new(format!(
                "invalid season {0}: must be in [{1}, {2}]",
                request.season,
                SchoolYear::MIN_START_YEAR,
                SchoolYear::MAX_START_YEAR
            ))
        })?;
        let run = CensusRun::new(season, request.revision).ok_or_else(|| {
            TerminalError::new(format!(
                "invalid run revision {0}: must be at least 1",
                request.revision
            ))
        })?;
        let query = requests::run_request(request);
        let evidence = self.store_evidence(&query).await?;
        let journal = super::open_work::measure(ctx, &query, &evidence).await?;
        let store = Arc::clone(self.jobs.store());
        let region = Arc::clone(self.jobs.region());
        let permit = self.jobs.permit_control().await?;
        let store_seal = requests::store_request(request, &journal, run);
        let reply = ctx
            .run(move || async move {
                super::blocking(region, move || {
                    let _permit = permit;
                    seal::seal(&store, &store_seal).map_err(|error| JobError::Terminal {
                        message: error.to_string(),
                    })
                })
                .await
                .and_then(|outcome| match (refusal, outcome.refusal.as_deref()) {
                    (RefusalPolicy::Retryable, Some(reason)) => Err(JobError::Transient {
                        message: format!("the census seal refused: {reason}"),
                    }),
                    _ => Ok(outcome),
                })
                .map(|outcome| Json(SealReply::of(&outcome)))
                .map_err(super::job_error)
            })
            .retry_policy(refusal.run_retry())
            .await?;
        Ok(reply)
    }
}

#[derive(Clone, Copy)]
enum RefusalPolicy {
    Reported,
    Retryable,
}

impl RefusalPolicy {
    fn run_retry(self) -> RunRetryPolicy {
        match self {
            Self::Reported => RunRetryPolicy::new().max_attempts(1),
            Self::Retryable => RunRetryPolicy::new()
                .initial_delay(Duration::from_secs(30))
                .exponentiation_factor(4.0)
                .max_delay(Duration::from_secs(300))
                .max_attempts(1),
        }
    }
}

#[service(
    journal_retention = "1 hour",
    idempotency_retention = "30 days",
    invocation_retry_policy(
        initial_interval = "500ms",
        max_interval = "1m",
        max_attempts = 3,
        on_max_attempts = "pause"
    )
)]
impl Census {
    #[handler]
    async fn status(&self, _ctx: Context<'_>) -> Result<Json<StatusReply>, HandlerError> {
        let stats = self.status_stats().await.map_err(job_error)?;
        let tables = stats
            .tables
            .iter()
            .map(|(table, rows)| TableCount {
                table: (*table).to_string(),
                rows: *rows,
            })
            .collect();
        Ok(Json(StatusReply {
            tables,
            observations: stats.observations,
            bytes_on_disk: stats.bytes_on_disk,
            today: self.clock.today(),
        }))
    }

    #[handler]
    async fn school_address_preflight(
        &self,
        ctx: Context<'_>,
        Json(request): Json<super::wire::SchoolAddressJoinRequest>,
    ) -> Result<Json<String>, HandlerError> {
        self.preflight_addresses(ctx, request).await
    }

    #[handler]
    async fn open_work(
        &self,
        ctx: Context<'_>,
        Json(request): Json<OpenWorkRequest>,
    ) -> Result<Json<OpenWorkReply>, HandlerError> {
        let evidence = self.store_evidence(&request).await?;
        Ok(Json(
            super::open_work::measure(&ctx, &request, &evidence).await?,
        ))
    }

    #[handler]
    #[tracing::instrument(skip_all, fields(grad_year = request.grad_year))]
    async fn seal(
        &self,
        ctx: Context<'_>,
        Json(request): Json<SealRequest>,
    ) -> Result<Json<SealReply>, HandlerError> {
        self.seal_reply(&ctx, &request, RefusalPolicy::Reported)
            .await
    }

    #[handler]
    #[tracing::instrument(skip_all, fields(grad_year = request.grad_year))]
    async fn publication(
        &self,
        ctx: Context<'_>,
        Json(request): Json<SealRequest>,
    ) -> Result<Json<SealReply>, HandlerError> {
        self.seal_reply(&ctx, &request, RefusalPolicy::Retryable)
            .await
    }

    #[handler]
    #[tracing::instrument(skip_all, fields(season = request.season, revision = request.revision))]
    async fn bind_run(
        &self,
        _ctx: Context<'_>,
        Json(request): Json<BindRunRequest>,
    ) -> Result<Json<BindRunReply>, HandlerError> {
        let season = SchoolYear::new(request.season).ok_or_else(|| {
            TerminalError::new(format!(
                "invalid season {0}: must be in [{1}, {2}]",
                request.season,
                SchoolYear::MIN_START_YEAR,
                SchoolYear::MAX_START_YEAR
            ))
        })?;
        let run = CensusRun::new(season, request.revision).ok_or_else(|| {
            TerminalError::new(format!(
                "invalid run revision {0}: must be at least 1",
                request.revision
            ))
        })?;
        let jurisdictions = requests::admitted_jurisdictions(&request.jurisdictions)?;
        let store = Arc::clone(&self.store);
        let region = Arc::clone(self.jobs.region());
        let permit = self.jobs.permit_control().await?;
        let result = super::blocking(region, move || {
            let _permit = permit;
            let identity = store_identity(&store).map_err(|error| JobError::Terminal {
                message: format!("store identity check failed: {error}"),
            })?;
            let manifest = RunManifest {
                store_identity: identity,
                run,
                cohort: GradYear::CO2027,
                jurisdictions,
            };
            store.bind_run(&manifest).map_err(|error| match error {
                census_store::StoreError::Refused { detail } => {
                    JobError::Terminal { message: detail }
                }
                other => JobError::Terminal {
                    message: format!("binding the run manifest failed: {other}"),
                },
            })?;
            Ok::<_, JobError>(BindRunReply {
                store_identity: manifest.store_identity,
                season: request.season,
                revision: request.revision,
                cohort: GradYear::CO2027.get(),
                jurisdictions: manifest.jurisdictions,
            })
        })
        .await
        .map_err(job_error)?;
        Ok(Json(result))
    }
}
