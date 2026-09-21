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
        let Some((probe_digest, probe)) =
            load_probe(ctx, runtime.clone(), &job, id, &mut state, &mut refs).await?
        else {
            continue;
        };
        let Some(probe_digest) =
            record_probe(&mut state, source_name.as_ref(), id, probe_digest, probe)
        else {
            continue;
        };
        complete_profile(
            ctx,
            runtime.clone(),
            &job,
            id,
            probe_digest,
            &mut state,
            &mut refs,
        )
        .await?;
    }
    Ok((state, refs))
}

/// Identifies one candidate and loads its probe artifact. `Ok(None)` records the row-level failure
/// in `state` and leaves that candidate's acquisition incomplete; a cancelled lane aborts.
async fn load_probe(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    job: &ProfileJob,
    id: AthleteId,
    state: &mut ProfileState,
    refs: &mut Vec<EvidenceDigest>,
) -> std::result::Result<Option<(EvidenceDigest, Box<ProfileProbe>)>, TerminalError> {
    let probe_digest = match identify(ctx, job).await {
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
            return Ok(None);
        }
    };
    refs.push(probe_digest.clone());
    let probe = match load_artifact::<ProfileProbe>(runtime, &probe_digest, state.bytes).await {
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
            return Ok(None);
        }
        Err(error) => {
            state.complete = false;
            state.coverage.push(CandidateCoverage::Incomplete {
                athlete_id: id,
                probe: Some(probe_digest),
                profile: None,
                issues: vec![format!("profile probe could not be decoded: {error}")],
            });
            return Ok(None);
        }
    };
    Ok(Some((probe_digest, probe)))
}

/// Retains the probe and records a name exclusion when one applies. The returned digest is `None`
/// exactly when the candidate needs no full profile acquisition.
fn record_probe(
    state: &mut ProfileState,
    source_name: Option<&CanonicalName>,
    id: AthleteId,
    probe_digest: EvidenceDigest,
    probe: Box<ProfileProbe>,
) -> Option<EvidenceDigest> {
    let candidate_exclusion = exclusion(source_name, &probe);
    state.probe_profiles.insert(id, probe.profiles);
    let Some(exclusion) = candidate_exclusion else {
        return Some(probe_digest);
    };
    state.exclusions.push(exclusion);
    let Some(source_name) = source_name.cloned() else {
        state.complete = false;
        state
            .issues
            .push("source canonical name is missing".to_owned());
        return None;
    };
    state.coverage.push(CandidateCoverage::NameExcluded {
        athlete_id: id,
        probe: probe_digest,
        source_name,
    });
    None
}

/// Gathers the full profile for one identified candidate and records it in `state`.
async fn complete_profile(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    job: &ProfileJob,
    id: AthleteId,
    probe_digest: EvidenceDigest,
    state: &mut ProfileState,
    refs: &mut Vec<EvidenceDigest>,
) -> std::result::Result<(), TerminalError> {
    let digest = match gather(ctx, job).await {
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
            return Ok(());
        }
    };
    refs.push(digest.clone());
    add_full_profile(state, id, probe_digest, digest, runtime).await;
    Ok(())
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
