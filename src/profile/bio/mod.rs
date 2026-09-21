//! Athletic.net bio (profile JSON) parsing.
//!
//! Identity, team, and sport-result evidence are assembled from a single envelope; a join
//! that cannot be resolved is reported as an `EvidenceIssue` instead of dropping the record.

mod distances;
mod events;
mod fields;
mod identity;
mod record;
mod relays;
mod results;
mod teams;

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{Map, Value};

use crate::domain::evidence::{ProfileEvidence, Sport};
use crate::domain::identity::{AthleteId, EvidenceDigest};
use crate::domain::name::BioIdentityObservation;

use fields::{availability, bounded_id, ev, issue, optional_text, retain_cap_issue};
use identity::{parse_identity, profile_url};
use results::parse_results;
use teams::parse_teams;

const MAX_BIO_BYTES: usize = 32 * 1024 * 1024;
const MAX_ITEMS: usize = 100_000;
const MAX_TEXT_BYTES: usize = 4_096;
const MAX_SOURCE_ID: u64 = 10_000_000_000_000;
const MAX_DISTANCE_DISPLAY: f64 = 10_000_000_000_000.0;
pub fn parse_bio(
    id: AthleteId,
    sport: Sport,
    digest: EvidenceDigest,
    bytes: &[u8],
) -> Result<ProfileEvidence> {
    if bytes.len() > MAX_BIO_BYTES {
        bail!("bio document exceeds {} bytes", MAX_BIO_BYTES);
    }
    let root: Value = serde_json::from_slice(bytes).context("bio JSON is invalid")?;
    let object = root
        .as_object()
        .ok_or_else(|| anyhow!("bio envelope is not an object"))?;
    parse_bio_value(id, sport, digest, object).map(|(profile, _)| profile)
}

pub(crate) fn parse_bio_value(
    id: AthleteId,
    sport: Sport,
    digest: EvidenceDigest,
    root: &Map<String, Value>,
) -> Result<(ProfileEvidence, Option<BioIdentityObservation>)> {
    let mut issues = Vec::new();
    let (name, identity) = parse_identity(root, id, sport, &digest, &mut issues)?;
    let (teams, grades) = parse_teams(root, &digest, &mut issues);
    let parsed = parse_results(root, id, sport, &digest, &mut issues);
    Ok((
        ProfileEvidence {
            athlete_id: id,
            profile_url: profile_url(id, sport)?,
            name,
            teams,
            graduation_years: Vec::new(),
            grades,
            sports: vec![availability(sport, parsed.count, parsed.present)],
            results: parsed.results,
            issues,
            documents: vec![digest],
        },
        identity,
    ))
}
