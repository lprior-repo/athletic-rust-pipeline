use super::{MAX_CANDIDATES, MAX_PROFILE_BYTES_PER_ROW};
use crate::{
    domain::{
        candidate::CandidateEvidence,
        evidence::ProfileEvidence,
        identity::{AthleteId, EvidenceDigest},
        name::{CanonicalName, NameExclusion},
    },
    model::SourceRecord,
    runtime::{
        acquisition::{ProfileAcquisition, ProfileJob, ProfileProbe, QueryEvidence, QueryJob},
        profile_worker::ProfileWorkerClient,
        query_worker::QueryWorkerClient,
        row_protocol::{CandidateCoverage, RankingDiscoveryEvidence},
        Runtime,
    },
    search::{SearchPage, SearchQuery},
};
use anyhow::{Context, Result};
use restate_sdk::prelude::*;
use serde::de::DeserializeOwned;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Debug, Default)]
pub(crate) struct DiscoveryState {
    pub incomplete: bool,
    pub refs: Vec<EvidenceDigest>,
    pub issues: Vec<String>,
    pub candidate_ids: BTreeSet<AthleteId>,
    pub candidate_limit: bool,
    pub rankings: Option<RankingDiscoveryEvidence>,
}

impl DiscoveryState {
    pub(crate) fn complete(&self) -> bool {
        !self.incomplete && !self.candidate_limit && self.issues.is_empty()
    }
}

#[derive(Debug, Default)]
pub(crate) struct ProfileState {
    pub profiles: Vec<ProfileEvidence>,
    pub probe_profiles: BTreeMap<AthleteId, Vec<ProfileEvidence>>,
    pub exclusions: Vec<NameExclusion>,
    pub coverage: Vec<CandidateCoverage>,
    pub issues: Vec<String>,
    pub complete: bool,
    bytes: usize,
}

impl ProfileState {
    pub(crate) fn complete(&self) -> bool {
        self.complete && self.issues.is_empty()
    }

    pub(crate) fn evidence(&self) -> impl Iterator<Item = CandidateEvidence<'_>> {
        self.coverage.iter().map(|coverage| {
            let athlete_id = coverage.athlete_id();
            let profile = self
                .profiles
                .iter()
                .find(|profile| profile.athlete_id == athlete_id);
            let incomplete = || CandidateEvidence::Incomplete {
                athlete_id,
                profile,
            };
            match coverage {
                CandidateCoverage::Complete { .. } => {
                    profile.map_or_else(incomplete, CandidateEvidence::Complete)
                }
                CandidateCoverage::Incomplete { .. } => incomplete(),
                CandidateCoverage::NameExcluded { .. } => self
                    .exclusions
                    .iter()
                    .find(|exclusion| exclusion.athlete_id() == athlete_id)
                    .zip(self.probe_profiles.get(&athlete_id))
                    .map_or_else(incomplete, |(exclusion, profiles)| {
                        CandidateEvidence::NameExcluded {
                            profiles,
                            exclusion,
                        }
                    }),
            }
        })
    }
}
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

pub(crate) async fn execute_profiles(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    snapshot: &EvidenceDigest,
    source: &SourceRecord,
    ids: &BTreeSet<AthleteId>,
) -> std::result::Result<(ProfileState, Vec<EvidenceDigest>), TerminalError> {
    let source_name = CanonicalName::from_source(source).ok().flatten();
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
        let probe_digest = match identify(ctx, &job).await {
            Ok(digest) => digest,
            Err(error) if error.code() == 409 => return Err(error),
            Err(error) => {
                state.complete = false;
                state.coverage.push(CandidateCoverage::Incomplete {
                    athlete_id: id,
                    probe: None,
                    profile: None,
                    issues: vec![format!("profile identify failed for {}: {error}", id.get())],
                });
                continue;
            }
        };
        refs.push(probe_digest.clone());
        let probe = match load_artifact::<ProfileProbe>(runtime.clone(), &probe_digest, state.bytes)
            .await
        {
            Ok(ProfileLoad::Loaded { artifact, bytes }) => {
                state.bytes = state.bytes.saturating_add(bytes);
                artifact
            }
            Ok(ProfileLoad::Limited { bytes }) => {
                state.complete = false;
                state.coverage.push(CandidateCoverage::Incomplete {
                    athlete_id: id,
                    probe: Some(probe_digest),
                    profile: None,
                    issues: vec![format!("profile probe exceeds row budget at {bytes} bytes")],
                });
                continue;
            }
            Err(error) => {
                state.complete = false;
                state.coverage.push(CandidateCoverage::Incomplete {
                    athlete_id: id,
                    probe: Some(probe_digest),
                    profile: None,
                    issues: vec![format!("profile probe could not be decoded: {error}")],
                });
                continue;
            }
        };
        let candidate_exclusion = exclusion(source_name.as_ref(), &probe);
        state.probe_profiles.insert(id, probe.profiles);
        if let Some(exclusion) = candidate_exclusion {
            state.exclusions.push(exclusion);
            let Some(source_name) = source_name.clone() else {
                state.complete = false;
                state
                    .issues
                    .push("source canonical name is missing".to_owned());
                continue;
            };
            state.coverage.push(CandidateCoverage::NameExcluded {
                athlete_id: id,
                probe: probe_digest,
                source_name,
            });
            continue;
        }
        let digest = match gather(ctx, &job).await {
            Ok(digest) => digest,
            Err(error) if error.code() == 409 => return Err(error),
            Err(error) => {
                state.complete = false;
                state.coverage.push(CandidateCoverage::Incomplete {
                    athlete_id: id,
                    probe: Some(probe_digest),
                    profile: None,
                    issues: vec![format!("profile gather failed for {}: {error}", id.get())],
                });
                continue;
            }
        };
        refs.push(digest.clone());
        add_full_profile(&mut state, id, probe_digest, digest, runtime.clone()).await;
    }
    Ok((state, refs))
}

fn exclusion(source: Option<&CanonicalName>, probe: &ProfileProbe) -> Option<NameExclusion> {
    let source = source?;
    if !probe.failures.is_empty() || probe.identities.len() != 2 || probe.html.is_none() {
        return None;
    }
    let html = probe.html.as_ref()?;
    NameExclusion::new(
        source.clone(),
        &probe.identities,
        crate::domain::name::HtmlIdentity {
            athlete_id: html.athlete_id,
            profile_url: &html.profile_url,
            names: &html.identity_hints,
            issues: &html.issues,
            document: &html.document,
        },
    )
    .ok()
}

async fn add_full_profile(
    state: &mut ProfileState,
    id: AthleteId,
    probe: EvidenceDigest,
    digest: EvidenceDigest,
    runtime: Arc<Runtime>,
) {
    match load_artifact::<ProfileAcquisition>(runtime, &digest, state.bytes).await {
        Ok(ProfileLoad::Loaded { artifact, bytes }) => {
            state.bytes = state.bytes.saturating_add(bytes);
            let complete =
                artifact.complete && artifact.failures.is_empty() && artifact.profile.is_some();
            if let Some(profile) = artifact.profile {
                state.profiles.push(profile);
            }
            if complete {
                state.coverage.push(CandidateCoverage::Complete {
                    athlete_id: id,
                    probe,
                    profile: digest,
                });
            } else {
                state.complete = false;
                state.coverage.push(CandidateCoverage::Incomplete {
                    athlete_id: id,
                    probe: Some(probe),
                    profile: Some(digest),
                    issues: artifact
                        .failures
                        .iter()
                        .map(|failure| failure.message.clone())
                        .collect(),
                });
            }
        }
        Ok(ProfileLoad::Limited { bytes }) => {
            state.complete = false;
            state.coverage.push(CandidateCoverage::Incomplete {
                athlete_id: id,
                probe: Some(probe),
                profile: Some(digest),
                issues: vec![format!("profile exceeds row budget at {bytes} bytes")],
            });
        }
        Err(error) => {
            state.complete = false;
            state.coverage.push(CandidateCoverage::Incomplete {
                athlete_id: id,
                probe: Some(probe),
                profile: Some(digest),
                issues: vec![format!("profile could not be decoded: {error}")],
            });
        }
    }
}

async fn identify(
    ctx: &ObjectContext<'_>,
    job: &ProfileJob,
) -> std::result::Result<EvidenceDigest, TerminalError> {
    let key = job
        .key()
        .map_err(|error| TerminalError::new(error.to_string()))?;
    let call = ctx
        .object_client::<ProfileWorkerClient>(&key)
        .identify(Json(job.clone()))
        .call();
    let handle = call
        .invocation_handle()
        .await
        .map_err(|error| TerminalError::new(error.to_string()))?;
    match call.await {
        Ok(value) => Ok(value.0),
        Err(error) if error.code() == 409 => {
            handle.cancel();
            Err(error)
        }
        Err(error) => Err(error),
    }
}

async fn gather(
    ctx: &ObjectContext<'_>,
    job: &ProfileJob,
) -> std::result::Result<EvidenceDigest, TerminalError> {
    let key = job
        .key()
        .map_err(|error| TerminalError::new(error.to_string()))?;
    let call = ctx
        .object_client::<ProfileWorkerClient>(&key)
        .gather(Json(job.clone()))
        .call();
    let handle = call
        .invocation_handle()
        .await
        .map_err(|error| TerminalError::new(error.to_string()))?;
    match call.await {
        Ok(value) => Ok(value.0),
        Err(error) if error.code() == 409 => {
            handle.cancel();
            Err(error)
        }
        Err(error) => Err(error),
    }
}

enum ProfileLoad<T> {
    Loaded { artifact: Box<T>, bytes: usize },
    Limited { bytes: usize },
}

async fn load_artifact<T: DeserializeOwned + Send + 'static>(
    runtime: Arc<Runtime>,
    digest: &EvidenceDigest,
    used: usize,
) -> Result<ProfileLoad<T>> {
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
            let artifact = serde_json::from_slice(&bytes).context("decoding profile artifact")?;
            Ok(ProfileLoad::Loaded {
                artifact: Box::new(artifact),
                bytes: bytes.len(),
            })
        })
        .await
}
