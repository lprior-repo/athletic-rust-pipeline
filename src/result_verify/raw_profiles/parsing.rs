use super::source_receipts;
use crate::{
    domain::{
        evidence::{EvidenceIssue, ProfileEvidence, Sport},
        identity::{AthleteId, ProfileUrl},
        name::BioIdentityObservation,
    },
    profile,
    runtime::{acquisition::TeamRequest, profile_worker::team, protocol::SourceResource},
    store::ArtifactStore,
};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use url::Url;

const BIO_PATH: &str = "/api/v1/AthleteBio/GetAthleteBioData";
const TEAM_PATH: &str = "/api/v1/TeamNav/Team";

pub(super) enum ParsedOperation {
    Bio {
        sport: Sport,
        profile: Box<ProfileEvidence>,
        identity: Option<BioIdentityObservation>,
        requests: Vec<TeamRequest>,
        issues: Vec<EvidenceIssue>,
    },
    Html {
        profile_url: ProfileUrl,
        profile: profile::HtmlProfileEvidence,
    },
    Team {
        observation: team::TeamObservation,
    },
}

pub(super) fn parse_observation(
    observation: &source_receipts::SourceObservation,
    athlete_id: AthleteId,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<ParsedOperation> {
    let receipt = &observation.receipt;
    let resource = classify(&Url::parse(&receipt.source_url)?, athlete_id)?;
    if !observation.matches(&resource, origin)? {
        bail!("captured source receipt does not match its production request resource");
    }
    let bytes = store
        .get_bytes(&receipt.digest)
        .context("reading hash-verified raw profile receipt")?;
    if u64::try_from(bytes.len())? != receipt.bytes {
        bail!("raw profile receipt byte count differs from retained bytes");
    }
    match resource {
        SourceResource::Bio { sport, .. } => {
            let root: serde_json::Map<String, Value> =
                serde_json::from_slice(&bytes).context("raw Bio receipt is invalid JSON")?;
            let (profile, identity) =
                profile::parse_bio_value(athlete_id, sport, receipt.digest.clone(), &root)?;
            let (requests, issues) =
                team::authorized_requests_value(&root, sport, &receipt.digest)?;
            Ok(ParsedOperation::Bio {
                sport,
                profile: Box::new(profile),
                identity,
                requests,
                issues,
            })
        }
        SourceResource::ProfileHtml { profile_url } => Ok(ParsedOperation::Html {
            profile_url,
            profile: profile::parse_profile_html(athlete_id, receipt.digest.clone(), &bytes)?,
        }),
        SourceResource::Team {
            team_id,
            sport,
            season,
        } => Ok(ParsedOperation::Team {
            observation: team::parse_team_nav(
                TeamRequest {
                    team_id,
                    sport,
                    season,
                },
                receipt.digest.clone(),
                &bytes,
            )?,
        }),
        SourceResource::Rankings { .. } => bail!("rankings not supported in raw profile parsing"),
        SourceResource::Search { .. } => bail!("raw profile receipt uses a search resource"),
    }
}

fn classify(url: &Url, athlete_id: AthleteId) -> Result<SourceResource> {
    if url.path() == BIO_PATH {
        let pairs = exact_query(url, &["athleteId", "sport", "level"])?;
        let id = query_value(&pairs, "athleteId")?.parse::<u64>()?;
        if id != athlete_id.get() || query_value(&pairs, "level")? != "0" {
            bail!("raw Bio receipt is not bound to the selected athlete and level");
        }
        let sport = match query_value(&pairs, "sport")? {
            "tf" => Sport::TrackField,
            "xc" => Sport::CrossCountry,
            _ => bail!("raw Bio receipt has an unsupported sport"),
        };
        return Ok(SourceResource::Bio { athlete_id, sport });
    }
    if url.path() == TEAM_PATH {
        let pairs = exact_query(url, &["team", "sport", "season"])?;
        let team_id = query_value(&pairs, "team")?.parse::<u64>()?;
        let sport = match query_value(&pairs, "sport")? {
            "tf" => Sport::TrackField,
            "xc" => Sport::CrossCountry,
            _ => bail!("TeamNav receipt has an unsupported sport"),
        };
        let season = query_value(&pairs, "season")?.parse::<u16>()?;
        if team_id == 0 || season == 0 {
            bail!("TeamNav request identifiers must be nonzero");
        }
        return Ok(SourceResource::Team {
            team_id,
            sport,
            season,
        });
    }
    let canonical = format!("https://athletic.net{}", url.path());
    let profile_url = ProfileUrl::parse(&canonical)?;
    if profile_url.athlete_id() != athlete_id || url.query().is_some() {
        bail!("raw HTML receipt is not bound to the selected athlete");
    }
    Ok(SourceResource::ProfileHtml { profile_url })
}

fn exact_query(url: &Url, keys: &[&str]) -> Result<Vec<(String, String)>> {
    let pairs = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    if pairs.len() != keys.len()
        || keys
            .iter()
            .any(|key| pairs.iter().filter(|(name, _)| name == key).count() != 1)
    {
        bail!("source receipt query parameters are not exact");
    }
    Ok(pairs)
}

fn query_value<'a>(pairs: &'a [(String, String)], key: &str) -> Result<&'a str> {
    pairs
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
        .context("source receipt query parameter is absent")
}
