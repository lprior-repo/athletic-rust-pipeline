use std::path::PathBuf;
use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::census::seal::{self, JournalCounts, SealRequest as StoreSealRequest};
use census_report::report::Scope;
use census_store::clock::Clock;
use census_store::Store;

use super::publish::Jobs;
use super::wire::{
    OpenWorkReply, OpenWorkRequest, SealReply, SealRequest, StatusReply, TableCount,
};
use super::JobError;

/// `Census`: the operator's read surface over the store, and the seal over both the store and the run.
///
/// The four heavy jobs are workflows of their own — see [`Jobs`](super::publish::Jobs) — because a
/// job is worth finishing: its journal and completion are retained, so a caller that repeats the job
/// attaches to the result instead of running months of work again.
///
/// A read has no such need. It answers from the store in milliseconds, and retaining an invocation
/// per status check would be retention with nothing behind it. The seal is the exception that earns
/// its place here: it *is* a read of the store, but only a context inside the service can read the
/// run's own objects, and §70 asks about both in one breath.
#[derive(Clone)]
pub struct Census {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
    /// The heavy-job region and permit. The seal reads the whole store, so it is started through the
    /// same spawner and capped by the same semaphore as the four jobs: nothing this service begins
    /// outlives the drain, and a seal cannot run beside four merges and starve them.
    jobs: Jobs,
}

impl Census {
    pub fn new(store: Arc<Store>, clock: Arc<dyn Clock>, jobs: Jobs) -> Self {
        Self { store, clock, jobs }
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

    /// The durable run's own open work: which jurisdiction sweeps still owe stages, and which source
    /// objects have accepted nothing.
    ///
    /// A read like [`Self::status`], but from the workflow's objects rather than from the store:
    /// these two counts are properties of the run, and the objects that did the work are the only
    /// surface that records them. The season and revision name the run, so a caller reads one run's
    /// work and not another's, and a count nobody could take comes back `None` rather than as zero.
    #[handler]
    async fn open_work(
        &self,
        ctx: Context<'_>,
        Json(request): Json<OpenWorkRequest>,
    ) -> Result<Json<OpenWorkReply>, HandlerError> {
        Ok(Json(super::open_work::measure(&ctx, &request).await?))
    }

    /// The §70 seal, assembled where both the store and the run's journal can be read.
    ///
    /// The offline seal holds the store and cannot read the journal: a jurisdiction's stages and a
    /// source object's accepted observations are recorded in the run's own objects, and only a
    /// context inside the service can address them. Assembled here, those two counts are *measured*
    /// instead of absent, which is what lets §70's first two items be certified at all — and the
    /// store-side counts stay where they were, because the store is the authority on its own rows.
    ///
    /// Idempotent: the evidence is a function of the store, the workbook and the journal, so the same
    /// census seals to the same digest, and a repeated `--write` writes the same bytes. Heavy — it
    /// walks the whole store — so it runs on the blocking pool under the shared load permit.
    #[handler]
    #[tracing::instrument(skip_all, fields(grad_year = request.grad_year))]
    async fn seal(
        &self,
        ctx: Context<'_>,
        Json(request): Json<SealRequest>,
    ) -> Result<Json<SealReply>, HandlerError> {
        let journal = super::open_work::measure(&ctx, &run_request(&request)).await?;
        let store = Arc::clone(self.jobs.store());
        let region = Arc::clone(self.jobs.region());
        let permit = self.jobs.permit().await?;
        let request = store_request(&request, &journal);
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
}

/// The run whose journal supplies the two counts the store cannot answer, by its own identity.
fn run_request(request: &SealRequest) -> OpenWorkRequest {
    OpenWorkRequest {
        season: request.season,
        revision: request.revision,
        source_objects: request.source_objects.clone(),
    }
}

/// The store-side seal request, carrying what the journal could measure and nothing it could not.
///
/// `source_failures` stays unmeasured: the journal reports the objects with no terminal acquisition,
/// which is what §70 asks for, and the per-attempt failure count is not a row either side keeps. The
/// objects that finished their walk empty are named alongside that count rather than inside it, so
/// the seal records them as findings instead of silently reading their sources as never read.
fn store_request(request: &SealRequest, journal: &OpenWorkReply) -> StoreSealRequest {
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
    }
}
