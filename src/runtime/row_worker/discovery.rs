use super::{MAX_CANDIDATES, MAX_PROFILE_BYTES_PER_ROW};
use crate::{
    domain::{
        evidence::ProfileEvidence,
        identity::{AthleteId, EvidenceDigest},
    },
    runtime::{
        acquisition::{ProfileAcquisition, ProfileJob, QueryEvidence, QueryJob},
        profile_worker::ProfileWorkerClient,
        query_worker::QueryWorkerClient,
        Runtime,
    },
    search::{SearchPage, SearchQuery},
};
use anyhow::{Context, Result};
use restate_sdk::prelude::*;
use std::{collections::BTreeSet, sync::Arc};

#[derive(Debug, Default)]
pub(crate) struct DiscoveryState {
    pub incomplete: bool,
    pub refs: Vec<EvidenceDigest>,
    pub issues: Vec<String>,
    pub candidate_ids: BTreeSet<AthleteId>,
    pub candidate_limit: bool,
}

impl DiscoveryState {
    pub(crate) fn complete(&self) -> bool {
        !self.incomplete && !self.candidate_limit && self.issues.is_empty()
    }
}

#[derive(Debug, Default)]
pub(crate) struct ProfileState {
    pub profiles: Vec<ProfileEvidence>,
    pub issues: Vec<String>,
    pub complete: bool,
    bytes: usize,
}

impl ProfileState {
    pub(crate) fn complete(&self) -> bool {
        self.complete && self.issues.is_empty()
    }
}

pub(crate) async fn execute_queries(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    snapshot: &EvidenceDigest,
    queries: Vec<SearchQuery>,
) -> DiscoveryState {
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
        let digest = match ctx
            .object_client::<QueryWorkerClient>(&key)
            .gather(Json(job.clone()))
            .call()
            .await
        {
            Ok(value) => value.0,
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
                artifact.issues.iter().for_each(|issue| {
                    state
                        .issues
                        .push(format!("query issue {}: {}", issue.code, issue.message))
                });
                artifact.failures.iter().for_each(|failure| {
                    state.issues.push(format!(
                        "query failure {:?}: {}",
                        failure.code, failure.message
                    ))
                });
                state.incomplete |= !artifact.complete;
            }
            Err(error) => state
                .issues
                .push(format!("query artifact could not be decoded: {error}")),
        }
    }
    state
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

pub(crate) async fn execute_profiles(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    snapshot: &EvidenceDigest,
    ids: &BTreeSet<AthleteId>,
) -> (ProfileState, Vec<EvidenceDigest>) {
    let mut state = ProfileState {
        complete: true,
        ..ProfileState::default()
    };
    let mut refs = Vec::new();
    for &id in ids {
        let job = ProfileJob {
            snapshot: snapshot.clone(),
            athlete_id: id,
        };
        let key = match job.key() {
            Ok(value) => value,
            Err(error) => {
                state.complete = false;
                state
                    .issues
                    .push(format!("profile key validation failed: {error}"));
                continue;
            }
        };
        let digest = match ctx
            .object_client::<ProfileWorkerClient>(&key)
            .gather(Json(job))
            .call()
            .await
        {
            Ok(value) => value.0,
            Err(error) => {
                state.complete = false;
                state.issues.push(format!(
                    "profile worker call failed for {}: {error}",
                    id.get()
                ));
                continue;
            }
        };
        refs.push(digest.clone());
        match load_profile(runtime.clone(), &digest, state.bytes).await {
            Ok(ProfileLoad::Loaded { artifact, bytes }) => {
                state.bytes = state.bytes.saturating_add(bytes);
                if !artifact.complete || !artifact.failures.is_empty() || artifact.profile.is_none()
                {
                    state.complete = false;
                    state
                        .issues
                        .push(format!("profile acquisition incomplete for {}", id.get()));
                }
                artifact.failures.iter().for_each(|failure| {
                    state.issues.push(format!(
                        "profile failure for {}: {}",
                        id.get(),
                        failure.message
                    ))
                });
                if let Some(profile) = artifact.profile {
                    state.profiles.push(profile);
                }
            }
            Ok(ProfileLoad::Limited { bytes }) => {
                state.complete = false;
                state.issues.push(format!("profile deserialization limit exceeded at {bytes} bytes; row budget is {MAX_PROFILE_BYTES_PER_ROW} bytes"));
            }
            Err(error) => {
                state.complete = false;
                state.issues.push(format!(
                    "profile artifact could not be decoded for {}: {error}",
                    id.get()
                ));
            }
        }
    }
    (state, refs)
}

enum ProfileLoad {
    Loaded {
        artifact: Box<ProfileAcquisition>,
        bytes: usize,
    },
    Limited {
        bytes: usize,
    },
}

async fn load_profile(
    runtime: Arc<Runtime>,
    digest: &EvidenceDigest,
    used: usize,
) -> Result<ProfileLoad> {
    let store = runtime.store.clone();
    let digest = digest.clone();
    runtime
        .blocking(move || {
            let bytes = store.get_bytes(&digest)?;
            if bytes.len() > MAX_PROFILE_BYTES_PER_ROW
                || used
                    .checked_add(bytes.len())
                    .is_none_or(|total| total > MAX_PROFILE_BYTES_PER_ROW)
            {
                return Ok(ProfileLoad::Limited { bytes: bytes.len() });
            }
            let artifact =
                serde_json::from_slice(&bytes).context("decoding profile acquisition artifact")?;
            Ok(ProfileLoad::Loaded {
                artifact,
                bytes: bytes.len(),
            })
        })
        .await
}
