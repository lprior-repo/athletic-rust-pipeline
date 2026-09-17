use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use std::collections::BTreeSet;

use super::super::acquisition::TeamRequest;
use crate::domain::evidence::{EvidenceIssue, EvidenceRef, Sport};
use crate::domain::facts::{Location, SchoolName};
use crate::domain::identity::EvidenceDigest;
use crate::profile::parse_location;

#[derive(Debug, Clone)]
pub(crate) struct TeamObservation {
    pub(crate) requested: TeamRequest,
    pub(crate) name: SchoolName,
    pub(crate) location: Option<Location>,
    pub(crate) level: Option<u8>,
    pub(crate) evidence: EvidenceRef,
}

const MAX_ENTRIES: usize = 100_000;
const MAX_TEAM_ID: u64 = 10_000_000_000_000;

pub(crate) fn authorized_requests_value(
    object: &serde_json::Map<String, Value>,
    sport: Sport,
    digest: &EvidenceDigest,
) -> Result<(Vec<TeamRequest>, Vec<EvidenceIssue>)> {
    let teams = object.get("allTeams").and_then(Value::as_object);
    let seasons = object.get("allSeasons").and_then(Value::as_array);
    let mut issues = Vec::new();
    if teams.is_none() {
        issues.push(issue(
            "unknown_teams_shape",
            "allTeams is absent or not an object",
            digest,
            "/allTeams".to_owned(),
        ));
    }
    if object
        .get("allSeasons")
        .is_some_and(|value| !value.is_array() && !value.is_null())
    {
        issues.push(issue(
            "unknown_seasons_shape",
            "allSeasons is not an array",
            digest,
            "/allSeasons".to_owned(),
        ));
    }
    if teams.is_some_and(|items| items.len() > MAX_ENTRIES) {
        issues.push(issue(
            "teams_truncated",
            "allTeams exceeds retained entry limit",
            digest,
            "/allTeams".to_owned(),
        ));
    }
    if seasons.is_some_and(|items| items.len() > MAX_ENTRIES) {
        issues.push(issue(
            "seasons_truncated",
            "allSeasons exceeds retained entry limit",
            digest,
            "/allSeasons".to_owned(),
        ));
    }
    let ids = team_ids(teams, digest, &mut issues);
    let requests = season_requests(seasons, sport, &ids, digest, &mut issues);
    Ok((requests, issues))
}

fn team_ids(
    teams: Option<&serde_json::Map<String, Value>>,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> BTreeSet<u64> {
    teams
        .into_iter()
        .flat_map(|map| map.keys().take(MAX_ENTRIES))
        .filter_map(|key| {
            match key
                .parse::<u64>()
                .ok()
                .filter(|id| (1..=MAX_TEAM_ID).contains(id))
            {
                Some(id) => Some(id),
                None => {
                    issues.push(issue(
                        "invalid_team_id",
                        "allTeams key is not a positive integer",
                        digest,
                        format!("/allTeams/{key}"),
                    ));
                    None
                }
            }
        })
        .collect()
}

fn season_requests(
    seasons: Option<&Vec<Value>>,
    sport: Sport,
    ids: &BTreeSet<u64>,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> Vec<TeamRequest> {
    seasons
        .into_iter()
        .flat_map(|items| items.iter().take(MAX_ENTRIES).enumerate())
        .filter_map(|(index, value)| {
            let object = value.as_object();
            let team = object
                .and_then(|item| item.get("SchoolID"))
                .and_then(Value::as_u64);
            let season = object
                .and_then(|item| item.get("IDSeason"))
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok());
            match (team, season) {
                (Some(team_id), Some(season)) if ids.contains(&team_id) && season > 0 => {
                    Some(TeamRequest {
                        team_id,
                        sport,
                        season,
                    })
                }
                _ => {
                    issues.push(issue(
                        "invalid_team_join",
                        "allSeasons entry is not an authorized allTeams join",
                        digest,
                        format!("/allSeasons/{index}"),
                    ));
                    None
                }
            }
        })
        .collect()
}

pub(crate) fn parse_team_nav(
    requested: TeamRequest,
    digest: EvidenceDigest,
    bytes: &[u8],
) -> Result<TeamObservation> {
    let root: Value = serde_json::from_slice(bytes).context("TeamNav response is invalid JSON")?;
    let team = root
        .get("team")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("TeamNav response has no team object"))?;
    let observed_id = team
        .get("ID")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("TeamNav team.ID is not an unsigned integer"))?;
    if observed_id != requested.team_id {
        bail!(
            "TeamNav team.ID {} does not match requested {}",
            observed_id,
            requested.team_id
        );
    }
    let raw_name = team
        .get("Name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("TeamNav team.Name is missing"))?;
    let name = SchoolName::parse(raw_name).context("TeamNav team.Name is invalid")?;
    let location = parse_location(team).context("TeamNav team location is invalid")?;
    let level = team
        .get("Level")
        .and_then(Value::as_u64)
        .map(|value| u8::try_from(value).map_err(|_| anyhow!("TeamNav team.Level exceeds u8")))
        .transpose()?;
    Ok(TeamObservation {
        requested,
        name,
        location,
        level,
        evidence: EvidenceRef {
            document: digest,
            locator: "/team".to_owned(),
        },
    })
}

fn issue(code: &str, message: &str, digest: &EvidenceDigest, locator: String) -> EvidenceIssue {
    EvidenceIssue {
        code: code.to_owned(),
        message: message.to_owned(),
        evidence: Some(EvidenceRef {
            document: digest.clone(),
            locator,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest() -> EvidenceDigest {
        EvidenceDigest::parse(&"a".repeat(64)).expect("synthetic digest")
    }

    #[test]
    fn team_nav_rejects_foreign_identifier() {
        let requested = TeamRequest {
            team_id: 7,
            sport: Sport::TrackField,
            season: 12025,
        };
        let digest = digest();
        let body = br#"{"team":{"ID":8,"Name":"Synthetic High","Level":4,"City":"Testville","State":"CA"}}"#;
        assert!(parse_team_nav(requested, digest, body).is_err());
    }

    #[test]
    fn authorized_requests_accepts_source_season_identifier() {
        let digest = digest();
        let body = br#"{"allTeams":{"7":{"SchoolName":"Synthetic High"}},"allSeasons":[{"SchoolID":7,"IDSeason":12025}]}"#;
        let object = serde_json::from_slice(body).expect("synthetic JSON");
        let result = authorized_requests_value(&object, Sport::TrackField, &digest);
        assert!(result.is_ok_and(|(requests, _)| requests
            .first()
            .is_some_and(|request| request.season == 12025)));
    }
}
