//! Team affiliations, grades, and locations.

use serde_json::{Map, Value};
use std::collections::BTreeMap;

use crate::domain::evidence::{EvidenceIssue, GradeAtSeason, Observed, TeamEvidence};
use crate::domain::facts::SchoolName;
use crate::domain::identity::EvidenceDigest;
use crate::profile::parse_location;

use super::fields::{bounded_id, ev, issue, retain_cap_issue};
use super::MAX_ITEMS;

pub(super) fn parse_teams(
    root: &Map<String, Value>,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> (Vec<TeamEvidence>, Vec<GradeAtSeason>) {
    let mut affiliations: BTreeMap<u64, Vec<u16>> = BTreeMap::new();
    if root
        .get("allSeasons")
        .is_some_and(|value| !value.is_array() && !value.is_null())
    {
        issues.push(issue(
            "unknown_seasons_shape",
            "allSeasons is not an array",
            Some(ev(digest, "/allSeasons")),
        ));
    }
    if let Some(items) = root.get("allSeasons").and_then(Value::as_array) {
        retain_cap_issue(
            items.len(),
            "seasons_truncated",
            "/allSeasons",
            digest,
            issues,
        );
    }
    root.get("allSeasons")
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|items| items.iter().take(MAX_ITEMS))
        .enumerate()
        .for_each(|(index, item)| {
            let Some(obj) = item.as_object() else {
                issues.push(issue(
                    "unknown_season_shape",
                    "allSeasons item is not an object",
                    Some(ev(digest, format!("/allSeasons/{index}"))),
                ));
                return;
            };
            let team = obj.get("SchoolID").and_then(Value::as_u64);
            let season = obj
                .get("IDSeason")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok());
            match (team, season) {
                (Some(team), Some(season)) if bounded_id(team) => {
                    affiliations.entry(team).or_default().push(season);
                }
                _ => issues.push(issue(
                    "invalid_season",
                    "season affiliation has invalid identifiers",
                    Some(ev(digest, format!("/allSeasons/{index}"))),
                )),
            }
        });
    let teams = match root.get("allTeams").and_then(Value::as_object) {
        Some(map) => {
            retain_cap_issue(map.len(), "teams_truncated", "/allTeams", digest, issues);
            map.iter()
                .take(MAX_ITEMS)
                .filter_map(|(key, value)| parse_team(key, value, &affiliations, digest, issues))
                .collect()
        }
        None => {
            issues.push(issue(
                "unknown_teams_shape",
                "allTeams is absent or not an object",
                Some(ev(digest, "/allTeams")),
            ));
            Vec::new()
        }
    };
    (teams, parse_grades(root.get("grades"), digest, issues))
}

fn parse_team(
    key: &str,
    value: &Value,
    affiliations: &BTreeMap<u64, Vec<u16>>,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> Option<TeamEvidence> {
    let id = match key.parse::<u64>().ok().filter(|value| *value > 0) {
        Some(id) => id,
        None => {
            issues.push(issue(
                "invalid_team_id",
                "allTeams key is not a positive integer",
                Some(ev(digest, format!("/allTeams/{key}"))),
            ));
            return None;
        }
    };
    let Some(obj) = value.as_object() else {
        issues.push(issue(
            "unknown_team_shape",
            "allTeams item is not an object",
            Some(ev(digest, format!("/allTeams/{key}"))),
        ));
        return None;
    };
    let name = obj
        .get("SchoolName")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| SchoolName::parse(value).ok());
    let Some(name) = name else {
        issues.push(issue(
            "team_name_missing",
            "team has no valid SchoolName",
            Some(ev(digest, format!("/allTeams/{key}/SchoolName"))),
        ));
        return None;
    };
    let location = match parse_location(obj) {
        Ok(value) => value.map(|value| Observed {
            value,
            evidence: ev(digest, format!("/allTeams/{key}")),
        }),
        Err(error) => {
            issues.push(issue(
                "invalid_team_location",
                &error.to_string(),
                Some(ev(digest, format!("/allTeams/{key}"))),
            ));
            None
        }
    };
    Some(TeamEvidence {
        team_id: id,
        name: Observed {
            value: name,
            evidence: ev(digest, format!("/allTeams/{key}/SchoolName")),
        },
        location,
        seasons: affiliations.get(&id).map_or_else(Vec::new, Clone::clone),
        level: obj
            .get("Level")
            .and_then(Value::as_u64)
            .and_then(|value| u8::try_from(value).ok()),
    })
}

fn parse_grades(
    value: Option<&Value>,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> Vec<GradeAtSeason> {
    let Some(map) = value.and_then(Value::as_object) else {
        if value.is_some() {
            issues.push(issue(
                "unknown_grades_shape",
                "grades is not an object",
                Some(ev(digest, "/grades")),
            ));
        }
        return Vec::new();
    };
    retain_cap_issue(map.len(), "grades_truncated", "/grades", digest, issues);
    map.iter()
        .take(MAX_ITEMS)
        .filter_map(|(key, value)| {
            let parsed = key.split_once('_').and_then(|(team, season)| {
                team.parse::<u64>().ok().zip(season.parse::<u16>().ok())
            });
            let Some((team_id, season)) = parsed else {
                issues.push(issue(
                    "invalid_grade_key",
                    "grade key is not SchoolID_IDSeason",
                    Some(ev(digest, format!("/grades/{key}"))),
                ));
                return None;
            };
            let Some(grade) = value.as_u64().and_then(|item| u8::try_from(item).ok()) else {
                issues.push(issue(
                    "invalid_grade",
                    "grade is not an unsigned integer",
                    Some(ev(digest, format!("/grades/{key}"))),
                ));
                return None;
            };
            Some(GradeAtSeason {
                team_id,
                season,
                grade,
                evidence: ev(digest, format!("/grades/{key}")),
            })
        })
        .collect()
}
