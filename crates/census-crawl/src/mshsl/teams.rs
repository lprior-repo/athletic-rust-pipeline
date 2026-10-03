use census_domain::model::{CoachRole, Gender, Sport};
use serde::Deserialize;

use super::text::clean;
use super::MAX_TEAMS_PER_SCHOOL;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamNode {
    pub nid: String,
    pub title: String,
    pub alias: String,
}

impl TeamNode {
    pub fn page_url(&self) -> Option<String> {
        self.alias
            .starts_with('/')
            .then(|| format!("https://www.mshsl.org{}", self.alias))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CoachRecord {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "coach_level")]
    pub level: String,
    #[serde(default, rename = "field_email")]
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamCoaches {
    pub node: TeamNode,
    pub api_url: String,
    pub records: Vec<CoachRecord>,
}

#[derive(Debug, Deserialize)]
struct TeamsPayload {
    #[serde(default)]
    data: Vec<TeamNodeRow>,
}

#[derive(Debug, Deserialize)]
struct TeamNodeRow {
    #[serde(default)]
    attributes: TeamNodeAttributes,
}

#[derive(Debug, Default, Deserialize)]
struct TeamNodeAttributes {
    #[serde(default, rename = "drupal_internal__nid")]
    nid: Option<u64>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    path: Option<TeamPathAlias>,
}

#[derive(Debug, Deserialize)]
struct TeamPathAlias {
    #[serde(default)]
    alias: Option<String>,
}

pub fn team_sport(alias: &str) -> Option<(Sport, Gender)> {
    let lowered = alias.to_ascii_lowercase();
    let activity = lowered.split('/').filter(|part| !part.is_empty()).nth(2)?;
    let sport = if activity.contains("cross-country") {
        Sport::CrossCountry
    } else if activity.contains("indoor-track") {
        Sport::IndoorTrack
    } else if activity.contains("track-and-field") || activity.contains("track-field") {
        Sport::OutdoorTrack
    } else {
        return None;
    };
    let gender = if activity.contains("boys") {
        Gender::Boys
    } else if activity.contains("girls") {
        Gender::Girls
    } else {
        Gender::Mixed
    };
    Some((sport, gender))
}

pub fn select_team_nodes(nodes: &[TeamNode]) -> Vec<TeamNode> {
    let mut selected: Vec<TeamNode> = Vec::new();
    let mut seen: Vec<(Sport, Gender)> = Vec::new();
    for node in nodes {
        if selected.len() >= MAX_TEAMS_PER_SCHOOL {
            break;
        }
        let Some(key) = team_sport(&node.alias) else {
            continue;
        };
        if !seen.contains(&key) {
            seen.push(key);
            selected.push(node.clone());
        }
    }
    selected
}

pub fn parse_team_nodes(payload: &str) -> Vec<TeamNode> {
    let Ok(parsed) = serde_json::from_str::<TeamsPayload>(payload) else {
        return Vec::new();
    };
    parsed
        .data
        .into_iter()
        .filter_map(|row| {
            let nid = row.attributes.nid?;
            Some(TeamNode {
                nid: nid.to_string(),
                title: row.attributes.title
                    .map_or(Default::default(), core::convert::identity)
                    .trim()
                    .to_string(),
                alias: row.attributes.path
                    .and_then(|path| path.alias)
                    .map_or(Default::default(), core::convert::identity),
            })
        })
        .collect()
}

pub fn parse_coach_records(payload: &str) -> Vec<CoachRecord> {
    let Ok(rows) = serde_json::from_str::<Vec<CoachRecord>>(payload) else {
        return Vec::new();
    };
    rows.into_iter()
        .map(|mut row| {
            row.name = clean(&row.name);
            row.level = row.level.trim().to_string();
            row.email = row
                .email
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            row
        })
        .collect()
}

pub fn is_published_level(level: &str) -> bool {
    !matches!(
        level.trim().to_ascii_lowercase().as_str(),
        "non-mshsl coach" | "mshsl sub-coach"
    )
}

pub fn coach_role(level: &str) -> Option<CoachRole> {
    match level.trim().to_ascii_lowercase().as_str() {
        "head coach" => Some(CoachRole::HeadCoach),
        "assistant coach" => Some(CoachRole::AssistantCoach),
        _ => None,
    }
}
