//! The run's durable fan-out: load and validate the bound inputs, seal the rankings collection the
//! run reads through, then drive row jobs at the requested concurrency.
//!
//! Split out of `run.rs` so the SDK-facing object surface stays the admission point and every
//! journaled side effect of one run lives in one driver.

use super::super::{
    acquisition::ACQUISITION_REVISION,
    import::SourceManifest,
    rankings_collection::{self, RankingCollectionRef, RankingsCollectionStateClient},
    row_worker::RowWorkerClient,
    run_protocol::{RunRequest, SourceSnapshot},
    Runtime,
};
use super::{results::Results, source_rows::SourceRows, terminal};
use crate::domain::identity::EvidenceDigest;
use restate_sdk::prelude::*;
use std::collections::BTreeMap;

/// The two immutable inputs one run binds: the ingestion manifest and the source snapshot.
pub(super) struct RunInputs {
    pub(super) manifest: SourceManifest,
    pub(super) snapshot: SourceSnapshot,
}

/// Start (or resume) the rankings collection this run depends on, then wait for it to seal.
///
/// The sealed reference is written into both the durable progress record and the row source, so a
/// later page read and a later resume agree on which collection snapshot the run used.
pub(super) async fn seal_rankings_collection(
    ctx: &ObjectContext<'_>,
    request: &RunRequest,
    inputs: &RunInputs,
    work: (&mut SourceRows, &mut Results<'_, '_>),
) -> Result<(), HandlerError> {
    let (rows, results) = work;
    let Some(scope) = &inputs.snapshot.rankings else {
        return Ok(());
    };
    let collection = rankings_collection::collection_fingerprint(&scope.revision, &request.snapshot)?;
    let collection_req = rankings_collection::CollectionRequest {
        source_snapshot: request.snapshot.clone(),
    };
    let client = ctx.object_client::<RankingsCollectionStateClient>(collection.as_str());
    client.start_or_resume(Json(collection_req)).call().await?;
    let sealed = wait_for_collection_sealed(ctx, &collection).await?;
    // Update Results with sealed collection_ref
    results.update_collection_ref(sealed.clone())?;
    rows.update_rankings(sealed);
    Ok(())
}

pub(super) async fn validate_snapshot(runtime: &Runtime, request: &RunRequest) -> anyhow::Result<()> {
    if usize::from(request.concurrency.get()) > runtime.config.row_concurrency() {
        anyhow::bail!("requested row concurrency exceeds worker capacity");
    }
    let snapshot: SourceSnapshot = runtime.load_json(&request.snapshot).await?;
    if snapshot.revision != ACQUISITION_REVISION
        || snapshot.source_origin != runtime.config.source_origin().as_str()
    {
        anyhow::bail!("source snapshot does not bind this worker source contract");
    }
    if snapshot.label.is_empty()
        || snapshot.label.len() > 128
        || snapshot.label.chars().any(char::is_control)
    {
        anyhow::bail!("source snapshot label is invalid");
    }
    Ok(())
}

#[tracing::instrument(skip_all, fields(concurrency = %request.concurrency))]
pub(super) async fn drive(
    ctx: &ObjectContext<'_>,
    runtime: &Runtime,
    request: &RunRequest,
    work: (&mut SourceRows, &mut Results<'_, '_>),
) -> Result<(), HandlerError> {
    let (rows, results) = work;
    let mut futures = DurableFuturesUnordered::new();
    let mut jobs = BTreeMap::new();
    let result: Result<(), HandlerError> = async {
        let mut exhausted = false;
        loop {
            while !exhausted && futures.len() < usize::from(request.concurrency.get()) {
                match rows.next(runtime, request).await.map_err(terminal)? {
                    Some(job) => {
                        let key = job.key().map_err(terminal)?;
                        let future = ctx
                            .object_client::<RowWorkerClient>(&key)
                            .process(Json(job))
                            .call();
                        let handle = future.invocation_handle().await?;
                        let index = futures.push(future);
                        if jobs.insert(index, (key, handle)).is_some() {
                            return Err(terminal("duplicate durable future index"));
                        }
                    }
                    None => exhausted = true,
                }
            }
            let Some((index, outcome)) = futures.next().await? else {
                break;
            };
            let (job_key, _) = jobs
                .remove(&index)
                .ok_or_else(|| terminal("completion has no admitted source row"))?;
            results.record(&job_key, outcome?.0).await?;
        }
        if !jobs.is_empty() {
            return Err(terminal("run ended with unaccounted durable row calls"));
        }
        Ok(())
    }
    .await;
    if result.is_err() {
        jobs.values().for_each(|(_, handle)| handle.cancel());
        for _ in 0..futures.len() {
            futures.next().await?;
        }
    }
    result
}

/// Observe the other object's shared state through journaled SDK calls.
async fn wait_for_collection_sealed(
    ctx: &ObjectContext<'_>,
    collection: &EvidenceDigest,
) -> Result<RankingCollectionRef, HandlerError> {
    loop {
        let snapshot = ctx
            .object_client::<RankingsCollectionStateClient>(collection.as_str())
            .snapshot_ref()
            .call()
            .await?
            .0;
        if let Some(snapshot) = snapshot {
            return Ok(RankingCollectionRef {
                collection: collection.clone(),
                snapshot,
            });
        }
        ctx.sleep(std::time::Duration::from_secs(1)).await?;
    }
}
