use super::fetch::{gather, identify, load_artifact, ProfileLoad};
use crate::{
    domain::{
        candidate::CandidateEvidence,
        evidence::ProfileEvidence,
        identity::{AthleteId, EvidenceDigest},
        name::{CanonicalName, NameExclusion},
    },
    model::SourceRecord,
    runtime::{
        acquisition::{ProfileAcquisition, ProfileJob, ProfileProbe},
        row_protocol::CandidateCoverage,
        Runtime,
    },
};
use restate_sdk::prelude::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

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
