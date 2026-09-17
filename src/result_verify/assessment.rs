use super::{checks, DetailRow};
use crate::{
    domain::{
        candidate::CandidateEvidence,
        decision,
        evidence::ProfileEvidence,
        identity::AthleteId,
        name::{HtmlIdentity, NameExclusion},
    },
    runtime::{
        acquisition::{ProfileAcquisition, ProfileProbe},
        row_protocol::CandidateCoverage,
        row_worker::MAX_PROFILE_BYTES_PER_ROW,
    },
};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::io::Write;

pub(super) fn verify(
    row: &DetailRow,
    profiles: &[ProfileAcquisition],
    assessment: &checks::AssessmentWire,
) -> Result<()> {
    let report = row
        .report
        .as_ref()
        .context("assessment has no row report")?;
    let mut probes = row.identity_artifacts.iter();
    let mut acquisitions = row.profile_artifacts.iter().zip(profiles);
    let mut used = 0;
    let candidates = report
        .candidates
        .iter()
        .map(|coverage| {
            let probe = coverage
                .probe()
                .map(|_| probes.next().context("candidate probe is absent"))
                .transpose()?;
            let acquisition = coverage
                .profile()
                .map(|_| acquisitions.next().context("candidate profile is absent"))
                .transpose()?;
            bind_candidate(coverage, probe, acquisition, &mut used)
        })
        .collect::<Result<Vec<_>>>()?;
    if probes.next().is_some() || acquisitions.next().is_some() {
        bail!("assessment has unbound candidate artifacts");
    }
    let recomputed = decision::assess(
        &row.source,
        candidates.iter().map(Candidate::evidence),
        assessment.search.clone(),
    )?;
    checks::verify_value_matches(
        row.assessment.as_ref().context("assessment is absent")?,
        &recomputed,
        "canonical assessment",
    )
}

enum Candidate<'a> {
    Complete(&'a ProfileEvidence),
    Incomplete {
        athlete_id: AthleteId,
        profile: Option<&'a ProfileEvidence>,
    },
    NameExcluded {
        profiles: Vec<ProfileEvidence>,
        exclusion: NameExclusion,
    },
}

impl Candidate<'_> {
    fn evidence(&self) -> CandidateEvidence<'_> {
        match self {
            Self::Complete(profile) => CandidateEvidence::Complete(profile),
            Self::Incomplete {
                athlete_id,
                profile,
            } => CandidateEvidence::Incomplete {
                athlete_id: *athlete_id,
                profile: *profile,
            },
            Self::NameExcluded {
                profiles,
                exclusion,
            } => CandidateEvidence::NameExcluded {
                profiles,
                exclusion,
            },
        }
    }
}

fn bind_candidate<'a>(
    coverage: &CandidateCoverage,
    probe: Option<&Value>,
    acquisition: Option<(&Value, &'a ProfileAcquisition)>,
    used: &mut usize,
) -> Result<Candidate<'a>> {
    let probe_loaded = probe
        .map(|value| admit(value, used))
        .transpose()?
        .is_some_and(|loaded| loaded);
    if acquisition.is_some() && !probe_loaded {
        bail!("profile acquisition follows an unavailable or over-budget probe");
    }
    let profile = match acquisition {
        Some((value, artifact)) if admit(value, used)? => artifact.profile.as_ref(),
        _ => None,
    };
    match coverage {
        CandidateCoverage::Complete { .. } => Ok(Candidate::Complete(
            profile.context("complete candidate lacks an admitted profile")?,
        )),
        CandidateCoverage::Incomplete { athlete_id, .. } => Ok(Candidate::Incomplete {
            athlete_id: *athlete_id,
            profile,
        }),
        CandidateCoverage::NameExcluded { source_name, .. } => {
            if !probe_loaded {
                bail!("name exclusion uses an over-budget probe");
            }
            let probe = ProfileProbe::deserialize(probe.context("name exclusion has no probe")?)?;
            let html = probe.html.as_ref().context("name exclusion has no HTML")?;
            let exclusion = NameExclusion::new(
                source_name.clone(),
                &probe.identities,
                HtmlIdentity {
                    athlete_id: html.athlete_id,
                    profile_url: &html.profile_url,
                    names: &html.identity_hints,
                    issues: &html.issues,
                    document: &html.document,
                },
            )?;
            Ok(Candidate::NameExcluded {
                profiles: probe.profiles,
                exclusion,
            })
        }
    }
}

fn admit(value: &Value, used: &mut usize) -> Result<bool> {
    // Coverage has already bound each value to its exact typed CAS serialization.
    // Object-key order changes no byte count. Count without another document buffer,
    // replaying the runtime's cumulative metadata limit and its skipped-load behavior.
    let mut counter = ByteCount(0);
    serde_json::to_writer(&mut counter, value).context("counting candidate artifact bytes")?;
    let Some(next) = used.checked_add(counter.0) else {
        return Ok(false);
    };
    if next > MAX_PROFILE_BYTES_PER_ROW {
        return Ok(false);
    }
    *used = next;
    Ok(true)
}

struct ByteCount(usize);

impl Write for ByteCount {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self.0.checked_add(bytes.len()).ok_or_else(|| {
            std::io::Error::other("serialized candidate artifact byte count overflow")
        })?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
