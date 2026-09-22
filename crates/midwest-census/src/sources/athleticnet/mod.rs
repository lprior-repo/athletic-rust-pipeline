//! Athletic.net athlete-bio adapter — the owner-authorized Athletic.net source.
//!
//! # Access
//!
//! Athletic.net's robots policy (fetched 2026-09-21, the whole 40-line file) allows `/api/` for
//! `User-agent: *` and disallows `/Search.aspx`, and the host answers the collector's **own** user
//! agent with `200` — so this adapter identifies itself, sends no spoofed browser headers, and
//! never touches the disallowed search endpoint. Athlete ids therefore cannot be discovered here:
//! they arrive from the operator's registry file (`--input`), one `athlete_id` or
//! `athlete_id,ST` per line.
//!
//! # Endpoint contract (verified against live responses, 2026-09-21)
//!
//! `GET https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=<id>&sport=<tf|xc>&level=4`
//!
//! | payload | `sport=tf` | `sport=xc` |
//! |---|---|---|
//! | `resultsTF` | every track & field result | empty |
//! | `resultsXC` | absent | every cross-country result |
//! | `eventsTF` | `IDEvent` → event label | null |
//! | `meets` | `IDMeet` → `{MeetName, EndDate}` for its own rows | same, for its own rows |
//! | `grades` | `"<SchoolID>_<SeasonID>"` → grade | same |
//! | `allTeams` | `SchoolID` → `{IDSchool, SchoolName, …}` | same |
//! | `allSeasons` | `[{SchoolID, IDSeason, Display}]` | same |
//!
//! The two calls are not redundant and neither is optional: an athlete with 40 track results and 22
//! cross-country results returns only the first under `sport=tf` and only the second under
//! `sport=xc`. Nothing in either payload carries the indoor/outdoor split except
//! `allSeasons[].Display` (`"2026 Indoor"`, `"2026 Outdoor"`).
//!
//! # What this adapter refuses to guess
//!
//! A row is skipped — and counted, and named in the run report — when its season has no
//! `allSeasons` entry (the indoor/outdoor split is then unpublished), when its school has no
//! `allTeams` entry, when its meet has no `meets` entry, when the target carries no state (school
//! identity keys on state + name, and `"Springfield"` in two states is two schools), or when the
//! mark is a no-mark token. The refusal is the point: a mislabelled indoor/outdoor row is worse
//! than a counted gap.
//!
//! # Layout
//!
//! `parse` decodes published payloads, `map` mints canonical entities, `absorb` walks one
//! payload's rows into them, and `collect` drives the run and journals it. The registry contract
//! lives here too: `parse_targets` reads the operator's file and `read_registry` is the run's way in.

use crate::sources::{CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use std::collections::HashSet;

mod absorb;
mod collect;
mod map;
mod parse;

pub use collect::collect;
pub use parse::{parse_mark, Bio, BioAthlete, BioEvent, BioMeet, BioSeason, BioTeam, TfRow, XcRow};

/// Athlete bio endpoint; `sport` (`tf`/`xc`), `athleteId` and `level` are its parameters.
const BIO_ENDPOINT: &str = "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData";

/// Athletic.net's "high school" level selector.
const HIGH_SCHOOL_LEVEL: u32 = 4;

/// Bump when a parser change alters what an already-journaled bio yields.
const PARSE_VERSION: u32 = 1;

/// Adapter options.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Registry file: one `athlete_id` or `athlete_id,ST` per line; `#` comments and blank lines are
    /// ignored. The operator supplies it because the search endpoint is disallowed by robots.
    pub input: Option<String>,
    /// Cap the number of athletes processed (smoke runs).
    pub limit: Option<usize>,
    /// Ignore cached bodies and re-fetch.
    pub refresh: bool,
    /// ISO date stamped into evidence.
    pub observed_on: String,
    /// Jurisdiction applied to targets that carry none. Only unambiguous for a single-state batch.
    pub states: Vec<UsJurisdiction>,
}

/// One athlete to read, with the jurisdiction that disambiguates its school.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub athlete_id: u64,
    pub state: Option<UsJurisdiction>,
}

/// A registry refusal, carried verbatim in the error's detail.
///
/// The parity harness pins these messages (`golden/athleticnet__registry-refusals.json`), so the
/// text reaches the operator exactly as the adapter wrote it, without a variant prefix.
fn registry_refusal(detail: String) -> CrawlError {
    CrawlError::Invariant { detail }
}

/// Parse the operator's athlete registry.
///
/// Lines are `athlete_id` or `athlete_id,ST`; a single `--states` value fills in the state for
/// targets that name none, and two or more are refused as ambiguous rather than guessed between.
///
/// A per-line state is a USPS code or nothing: the registry is a vendor file, so a spelled-out name
/// is refused rather than accepted through the lenient jurisdiction parser.
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
        // A registry may legitimately repeat an athlete across state files; read it once.
        if seen.insert(athlete_id) {
            targets.push(Target { athlete_id, state });
        }
    }
    Ok(targets)
}

/// Read the operator's registry file into targets.
///
/// Refuses a missing `--input` (the registry is the only way in: the search endpoint that would
/// discover ids is disallowed by robots) and a registry that lists no athlete.
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

/// Which endpoint answered: track & field, or cross country.
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
