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
        let key = match job.key() {
            Ok(value) => value,
            Err(error) => {
                state
                    .issues
                    .push(format!("query key validation failed: {error}"));
                continue;
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
                continue;
            }
        };
        state.refs.push(digest.clone());
        match runtime.load_json::<QueryEvidence>(&digest).await {
            Ok(artifact) => {
                add_candidates(&runtime, &mut state, &artifact).await;
                state.issues.extend(
                    artifact
                        .issues
                        .iter()
                        .map(|issue| format!("query issue {}: {}", issue.code, issue.message)),
                );
                state.issues.extend(artifact.failures.iter().map(|failure| {
                    format!("query failure {:?}: {}", failure.code, failure.message)
                }));
                state.incomplete |= !artifact.complete;
            }
            Err(error) => state
                .issues
                .push(format!("query artifact could not be decoded: {error}")),
        }
    }
    Ok(state)
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
