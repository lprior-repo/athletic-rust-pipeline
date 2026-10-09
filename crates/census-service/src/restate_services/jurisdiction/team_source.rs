use restate_sdk::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use tokio::sync::Mutex;

use census_reconcile::identity::WorkflowIdentity;

use super::JurisdictionCensus;
use crate::restate_services::wire::{
    StageOutcome, TeamsAttemptProgress, TeamsSourceInspection, TeamsSourceOutcome,
    TeamsSourceRequest,
};
use crate::restate_services::{blocking, job_error, teams_arms, JobError, Jobs};

mod admission;
mod execution;
mod operation;
pub(crate) use operation::contacts_key;
mod ledger;
#[cfg(feature = "native-fault-injection")]
mod native_boundary;
#[cfg(feature = "native-fault-injection")]
pub use native_boundary::NATIVE_BOUNDARY_HOOK;
#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct TeamsSource {
    owner: JurisdictionCensus,
    jobs: Jobs,
    gates: Arc<Mutex<HashMap<String, Weak<Mutex<()>>>>>,
}

impl TeamsSource {
    pub fn new(owner: JurisdictionCensus, jobs: Jobs) -> Self {
        Self {
            owner,
            jobs,
            gates: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn register(
        &self,
        ctx: &ObjectContext<'_>,
        request: &Arc<TeamsSourceRequest>,
        admission: &Arc<admission::WorkAdmission>,
    ) -> Result<ledger::Identity, HandlerError> {
        let store = Arc::clone(&self.owner.store);
        let region = Arc::clone(self.jobs.region());
        let request = Arc::clone(request);
        let operation = ctx.key().to_string();
        let admission = Arc::clone(admission);
        let Json(identity) = ctx
            .run(move || async move {
                blocking(
                    region,
                    admission::registration_worker(admission, store, operation, request),
                )
                .await
                .map(Json)
                .map_err(job_error)
            })
            .await?;
        Ok(identity)
    }

    async fn acquire(&self, request: &TeamsSourceRequest, observed_on: String) -> ledger::Attempt {
        let fetcher = match self
            .owner
            .fetcher(
                &request.jurisdiction.authorized_hosts,
                request.jurisdiction.source_parallelism,
            )
            .await
        {
            Ok(fetcher) => fetcher,
            Err(error) => {
                return ledger::Attempt::Failed(JobError::Terminal {
                    message: format!("{error:?}"),
                })
            }
        };
        match teams_arms::team_source(
            teams_arms::SourceRuntime {
                store: &self.owner.store,
                fetcher: &fetcher,
                region: self.jobs.region(),
            },
            request.jurisdiction.jurisdiction,
            crate::restate_services::jobs::AdapterScope {
                season: request.jurisdiction.season,
                refresh: request.jurisdiction.refresh,
                at: &observed_on,
                as_of: request.jurisdiction.history.as_of(),
            },
            &request.source,
        )
        .await
        {
            Ok(report) => stage(report, observed_on),
            Err(error) => ledger::Attempt::Failed(error),
        }
    }
}

pub(super) fn key(request: &TeamsSourceRequest) -> String {
    let parent = WorkflowIdentity::jurisdiction(
        request.jurisdiction.jurisdiction,
        request.jurisdiction.season,
        request.jurisdiction.revision,
    );
    format!("{}/teams/{}", parent.as_str(), request.source)
}

fn stage(report: Option<census_crawl::AdapterReport>, at: String) -> ledger::Attempt {
    let Some(report) = report else {
        return ledger::Attempt::Failed(JobError::Terminal {
            message: "a non-teams source reached the teams acquisition handler".to_string(),
        });
    };
    let records = match usize::try_from(report.rows) {
        Ok(records) => records,
        Err(_) => {
            return ledger::Attempt::Failed(JobError::Terminal {
                message: format!(
                    "{} reported an uncountable row total {}",
                    report.adapter, report.rows
                ),
            })
        }
    };
    let outcome = report_outcome(report, records, at);
    if outcome.disposition.is_complete()
        && outcome.errors.is_empty()
        && outcome.unfinished.is_empty()
    {
        ledger::Attempt::Completed(outcome)
    } else {
        let message = format!("incomplete teams acquisition: {:?}", outcome.disposition);
        ledger::Attempt::Incomplete {
            outcome,
            error: JobError::Transient { message },
        }
    }
}

fn report_outcome(report: census_crawl::AdapterReport, records: usize, at: String) -> StageOutcome {
    let errors = if report.errors > 0
        || report.rejections > 0
        || report
            .unresolved
            .is_some_and(|value| value.rows > 0 || value.labels > 0)
    {
        vec![format!(
            "{} source errors, {} rejected rows; unresolved {:?}",
            report.errors, report.rejections, report.unresolved
        )]
    } else {
        Vec::new()
    };
    StageOutcome {
        records,
        at,
        errors,
        notes: report.notes,
        disposition: report.disposition,
        unfinished: report.unfinished,
    }
}

#[object(invocation_retry_policy(
    initial_interval = "500ms",
    max_interval = "1m",
    max_attempts = 3,
    on_max_attempts = "kill"
))]
impl TeamsSource {
    #[handler]
    async fn run(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<TeamsSourceRequest>,
    ) -> Result<Json<TeamsSourceOutcome>, HandlerError> {
        if operation::phase(ctx.key(), &request).map_err(job_error)? != operation::Phase::Teams
            || teams_arms::arm_for(&request.source).is_none()
        {
            return Err(TerminalError::new(
                "teams source request does not match its operation key",
            )
            .into());
        }
        self.execute(ctx, request).await
    }

    #[handler]
    async fn contacts(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<TeamsSourceRequest>,
    ) -> Result<Json<TeamsSourceOutcome>, HandlerError> {
        if operation::phase(ctx.key(), &request).map_err(job_error)?
            != operation::Phase::SchoolContacts
        {
            return Err(TerminalError::new(
                "school contact request does not match its operation key",
            )
            .into());
        }
        self.execute(ctx, request).await
    }

    #[handler]
    async fn state(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<Option<TeamsSourceOutcome>>, HandlerError> {
        let outcome = ledger::status(&self.owner.store, ctx.key(), None).map_err(job_error)?;
        Ok(Json(outcome))
    }

    #[handler]
    async fn progress(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<Vec<TeamsAttemptProgress>>, HandlerError> {
        let inspection =
            ledger::inspection(&self.owner.store, ctx.key(), None).map_err(job_error)?;
        let progress = match inspection {
            TeamsSourceInspection::Unsettled { progress } => progress,
            TeamsSourceInspection::Settled { outcome } => match outcome {
                TeamsSourceOutcome::Completed { progress, .. }
                | TeamsSourceOutcome::Terminal { progress, .. }
                | TeamsSourceOutcome::Exhausted { progress, .. }
                | TeamsSourceOutcome::Interrupted { progress, .. } => progress,
            },
        };
        Ok(Json(progress))
    }

    #[handler]
    async fn inspection(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<TeamsSourceInspection>, HandlerError> {
        let inspection =
            ledger::inspection(&self.owner.store, ctx.key(), None).map_err(job_error)?;
        Ok(Json(inspection))
    }
}
