//! Athlete identity: requested-ID match, name normalization, and profile URL.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{Map, Value};

use crate::domain::evidence::{EvidenceIssue, Observed, Sport};
use crate::domain::facts::AthleteName;
use crate::domain::identity::{AthleteId, EvidenceDigest, ProfileUrl};
use crate::domain::name::{BioIdentityObservation, CanonicalName};

use super::fields::{ev, issue};
use super::MAX_TEXT_BYTES;

pub(super) fn profile_url(id: AthleteId, sport: Sport) -> Result<ProfileUrl> {
    let suffix = match sport {
        Sport::TrackField => "track-and-field",
        Sport::CrossCountry => "cross-country",
    };
    ProfileUrl::parse(&format!(
        "https://www.athletic.net/athlete/{}/{suffix}",
        id.get()
    ))
    .context("profile URL is invalid")
}

pub(super) fn parse_identity(
    root: &Map<String, Value>,
    requested: AthleteId,
    sport: Sport,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> Result<(Observed<AthleteName>, Option<BioIdentityObservation>)> {
    let athlete = observed_athlete(root, requested)?;
    let (value, identity) = bio_identity(athlete, requested, sport, digest, issues)?;
    Ok((
        Observed {
            value,
            evidence: ev(digest, "/athlete/FirstName+/LastName"),
        },
        identity,
    ))
}

/// The embedded athlete object, once it is known to be the requested athlete.
fn observed_athlete(
    root: &Map<String, Value>,
    requested: AthleteId,
) -> Result<&Map<String, Value>> {
    let athlete = root
        .get("athlete")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("bio athlete object missing"))?;
    let observed = athlete
        .get("IDAthlete")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("athlete ID is not an unsigned integer"))?;
    if observed != requested.get() {
        bail!(
            "bio athlete ID {} does not match requested {}",
            observed,
            requested.get()
        );
    }
    Ok(athlete)
}

/// Combined display name plus the per-component observation, absent when a component is empty.
fn bio_identity(
    athlete: &Map<String, Value>,
    requested: AthleteId,
    sport: Sport,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> Result<(AthleteName, Option<BioIdentityObservation>)> {
    let first = athlete_text(athlete.get("FirstName"), "FirstName")?;
    let last = athlete_text(athlete.get("LastName"), "LastName")?;
    let first_trimmed = first.trim();
    let last_trimmed = last.trim();
    let value = AthleteName::parse(format!("{first_trimmed} {last_trimmed}").trim())
        .context("athlete name is invalid")?;
    let first_evidence = ev(digest, "/athlete/FirstName");
    let last_evidence = ev(digest, "/athlete/LastName");
    let identity_complete =
        CanonicalName::parse(first_trimmed).is_ok() && CanonicalName::parse(last_trimmed).is_ok();
    if !identity_complete {
        issues.push(issue(
            "identity_incomplete",
            "athlete name has an empty normalized component",
            Some(first_evidence.clone()),
        ));
    }
    let first_name = AthleteName::parse(&first).ok();
    let last_name = AthleteName::parse(&last).ok();
    let identity = match (first_name, last_name) {
        (Some(first), Some(last)) if identity_complete => Some(BioIdentityObservation {
            athlete_id: requested,
            sport,
            first: Observed {
                value: first,
                evidence: first_evidence,
            },
            last: Observed {
                value: last,
                evidence: last_evidence,
            },
        }),
        _ => None,
    };
    Ok((value, identity))
}

fn athlete_text(value: Option<&Value>, key: &str) -> Result<String> {
    let text = value
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("athlete {key} is not text"))?;
    if text.len() > MAX_TEXT_BYTES {
        bail!("athlete {key} exceeds {MAX_TEXT_BYTES} bytes");
    }
    Ok(text.to_owned())
}
