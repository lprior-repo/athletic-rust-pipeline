use crate::{CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use std::collections::HashSet;

mod absorb;
mod collect;
mod map;
mod meet;
mod parse;

pub use collect::collect;
pub use meet::{
    grade_of, jurisdiction_of, meet_requests, metadata_request, AllResults, EventDivisions,
    EventMetadata, FlatEvent, FlatRow, MeetData, PublishedEvent, PublishedLeg, PublishedMeet,
    PublishedTeam,
};
pub use parse::{parse_mark, Bio, BioAthlete, BioEvent, BioMeet, BioSeason, BioTeam, TfRow, XcRow};

const BIO_ENDPOINT: &str = "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData";

const HIGH_SCHOOL_LEVEL: u32 = 4;

pub const SCHOOL_KIND: &str = "school";

const PARSE_VERSION: u32 = 1;

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub input: Option<String>,
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub meets: Vec<i64>,
    pub event_metadata: bool,
    pub meet_limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub athlete_id: u64,
    pub state: Option<UsJurisdiction>,
}

fn registry_refusal(detail: String) -> CrawlError {
    CrawlError::Invariant { detail }
}

pub fn parse_targets(body: &str, default_states: &[UsJurisdiction]) -> CrawlResult<Vec<Target>> {
    if default_states.len() > 1 {
        let codes: Vec<&str> = default_states.iter().map(|state| state.code()).collect();
        return Err(registry_refusal(format!(
            "a registry without a per-line state needs exactly one --states value, got {} ({})",
            default_states.len(),
            codes.join(",")
        )));
    }
    let fallback = default_states.first().copied();
    let mut targets = Vec::new();
    let mut seen = HashSet::new();
    for (number, line) in body.lines().enumerate() {
        let line_number = number.saturating_add(1);
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split([',', '\t', ' ']).filter(|part| !part.is_empty());
        let id_text = parts.next().unwrap_or_default();
        let athlete_id: u64 = id_text.parse().map_err(|_| {
            registry_refusal(format!(
                "registry line {}: `{id_text}` is not an athlete id",
                line_number
            ))
        })?;
        let published = parts.next().map(|state| state.trim().to_ascii_uppercase());
        if parts.next().is_some() {
            return Err(registry_refusal(format!(
                "registry line {}: expected `athlete_id[,ST]`, got `{line}`",
                line_number
            )));
        }
        let state = match published {
            Some(raw) if raw.len() == 2 && raw.chars().all(|c| c.is_ascii_alphabetic()) => {
                Some(UsJurisdiction::from_code(&raw).ok_or_else(|| {
                    registry_refusal(format!(
                        "registry line {line_number}: `{raw}` is not one of the 50 states or the \
                         District of Columbia"
                    ))
                })?)
            }
            Some(raw) => {
                return Err(registry_refusal(format!(
                    "registry line {line_number}: `{raw}` is not a two-letter state code"
                )))
            }
            None => fallback,
        };
        if seen.insert(athlete_id) {
            targets.push(Target { athlete_id, state });
        }
    }
    Ok(targets)
}

fn read_registry(options: &Options) -> CrawlResult<Vec<Target>> {
    let Some(input) = options.input.as_deref() else {
        return Err(CrawlError::Invariant {
            detail: "the athletic.net adapter needs --input with an athlete registry; its search \
                     endpoint is disallowed by robots, so ids cannot be discovered by the tool"
                .to_string(),
        });
    };
    let body = std::fs::read_to_string(input).map_err(|source| CrawlError::Io {
        path: std::path::PathBuf::from(input),
        source,
    })?;
    let targets = parse_targets(&body, &options.states)?;
    if targets.is_empty() {
        return Err(CrawlError::Schema {
            url: input.to_string(),
            detail: "the athlete registry lists no athlete ids".to_string(),
        });
    }
    Ok(targets)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    TrackField,
    CrossCountry,
}

impl Scope {
    fn parameter(self) -> &'static str {
        match self {
            Scope::TrackField => "tf",
            Scope::CrossCountry => "xc",
        }
    }
}

const SCOPES: [Scope; 2] = [Scope::TrackField, Scope::CrossCountry];

#[cfg(test)]
mod tests;
