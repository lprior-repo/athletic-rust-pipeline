use std::path::PathBuf;
use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::census::seal::{self, census_cohort, JournalCounts, SealRequest as StoreSealRequest};
use census_domain::model::{CensusRun, GradYear, RunManifest, SchoolYear};
use census_domain::UsJurisdiction;
use census_reconcile::identity::admitted_scope;
use census_report::export::store_identity;
use census_report::report::Scope;
use census_store::clock::Clock;
use census_store::Store;

use super::publish::Jobs;
use super::wire::{
    BindRunReply, BindRunRequest, OpenWorkReply, OpenWorkRequest, SealReply, SealRequest,
    StatusReply, TableCount,
};
use super::{job_error, JobError};

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
        let permit = self.jobs.permit().await?;
        super::blocking(region, move || {
            let _permit = permit;
            super::open_work::inspect_store(&store, season, revision)
        })
        .await
        .map_err(job_error)
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
        let stats = self.store.stats()?;
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
        let query = run_request(&request);
        let evidence = self.store_evidence(&query).await?;
        let journal = super::open_work::measure(&ctx, &query, &evidence).await?;
        let store = Arc::clone(self.jobs.store());
        let region = Arc::clone(self.jobs.region());
        let permit = self.jobs.permit().await?;
        let request = store_request(&request, &journal, run);
        let reply = ctx
            .run(move || async move {
                super::blocking(region, move || {
                    let _permit = permit;
                    seal::seal(&store, &request).map_err(|error| JobError::Terminal {
                        message: error.to_string(),
                    })
                })
                .await
                .map(|outcome| Json(SealReply::of(&outcome)))
                .map_err(super::job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(reply)
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
        let jurisdictions = admitted_jurisdictions(&request.jurisdictions)?;
        let store = Arc::clone(&self.store);
        let region = Arc::clone(self.jobs.region());
        let permit = self.jobs.permit().await?;
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

fn admitted_jurisdictions(
    requested: &[UsJurisdiction],
) -> Result<Vec<UsJurisdiction>, HandlerError> {
    let admitted = admitted_scope(requested);
    for jurisdiction in &admitted {
        if let Err(outside) = jurisdiction.require_census_scope() {
            return Err(TerminalError::new(outside.to_string()).into());
        }
    }
    Ok(UsJurisdiction::CENSUS_SCOPE
        .iter()
        .copied()
        .filter(|state| admitted.contains(state))
        .collect())
}

fn run_request(request: &SealRequest) -> OpenWorkRequest {
    OpenWorkRequest {
        season: request.season,
        revision: request.revision,
        source_objects: request.source_objects.clone(),
    }
}

fn store_request(
    request: &SealRequest,
    journal: &OpenWorkReply,
    run: CensusRun,
) -> StoreSealRequest {
    StoreSealRequest {
        grad_year: request.grad_year,
        scope: if request.all_sources {
            Scope::AllSources
        } else {
            Scope::Core
        },
        workbook: request.workbook.as_ref().map(PathBuf::from),
        write: request.write,
        journal: Some(JournalCounts {
            jurisdiction_sweeps: journal.jurisdiction_sweeps,
            source_objects: journal.source_objects,
            silent_sources: journal.silent_sources.clone(),
        }),
        source_failures: None,
        run: Some(run),
    }
}
