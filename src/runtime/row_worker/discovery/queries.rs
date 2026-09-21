use super::super::MAX_CANDIDATES;
use super::DiscoveryState;
use crate::{
    domain::identity::EvidenceDigest,
    runtime::{
        acquisition::{QueryEvidence, QueryJob},
        query_worker::QueryWorkerClient,
        Runtime,
    },
    search::{SearchPage, SearchQuery},
};
use restate_sdk::prelude::*;
use std::sync::Arc;

pub(crate) async fn execute_queries(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    snapshot: &EvidenceDigest,
    queries: Vec<SearchQuery>,
) -> std::result::Result<DiscoveryState, TerminalError> {
    let mut state = DiscoveryState::default();
    for query in queries {
        let job = QueryJob {
            snapshot: snapshot.clone(),
            query,
        };
        run_query(ctx, &runtime, job, &mut state).await?;
    }
    Ok(state)
}

/// Runs one query worker call and folds its evidence into `state`. An invalid query key or a
/// failed call is recorded as an issue and leaves that query unaquired; a cancelled lane aborts.
async fn run_query(
    ctx: &ObjectContext<'_>,
    runtime: &Runtime,
    job: QueryJob,
    state: &mut DiscoveryState,
) -> std::result::Result<(), TerminalError> {
    let key = match job.key() {
        Ok(value) => value,
        Err(error) => {
            state
                .issues
                .push(format!("query key validation failed: {error}"));
            return Ok(());
        }
    };
    let call = ctx
        .object_client::<QueryWorkerClient>(&key)
        .gather(Json(job))
        .call();
    let handle = call
        .invocation_handle()
        .await
        .map_err(|error| TerminalError::new(error.to_string()))?;
    let digest = match call.await {
        Ok(value) => value.0,
        Err(error) if error.code() == 409 => {
            handle.cancel();
            return Err(error);
        }
        Err(error) => {
            state
                .issues
                .push(format!("query worker call failed: {error}"));
            return Ok(());
        }
    };
    state.refs.push(digest.clone());
    match runtime.load_json::<QueryEvidence>(&digest).await {
        Ok(artifact) => fold_query(runtime, state, &artifact).await,
        Err(error) => state
            .issues
            .push(format!("query artifact could not be decoded: {error}")),
    }
    Ok(())
}

/// Folds one acquired query artifact into `state`: candidates, issues and completeness.
async fn fold_query(runtime: &Runtime, state: &mut DiscoveryState, artifact: &QueryEvidence) {
    add_candidates(runtime, state, artifact).await;
    state.issues.extend(
        artifact
            .issues
            .iter()
            .map(|issue| format!("query issue {}: {}", issue.code, issue.message)),
    );
    state.issues.extend(
        artifact
            .failures
            .iter()
            .map(|failure| format!("query failure {:?}: {}", failure.code, failure.message)),
    );
    state.incomplete |= !artifact.complete;
}

async fn add_candidates(runtime: &Runtime, state: &mut DiscoveryState, artifact: &QueryEvidence) {
    for page in &artifact.pages {
        if state.candidate_limit {
            break;
        }
        match runtime.load_json::<SearchPage>(&page.parsed).await {
            Ok(search_page) => search_page.results().iter().for_each(|candidate| {
                if state.candidate_ids.contains(&candidate.id()) {
                    return;
                }
                if state.candidate_ids.len() == MAX_CANDIDATES {
                    state.candidate_limit = true;
                    return;
                }
                state.candidate_ids.insert(candidate.id());
            }),
            Err(error) => state.issues.push(format!(
                "search page artifact could not be decoded: {error}"
            )),
        }
    }
}
