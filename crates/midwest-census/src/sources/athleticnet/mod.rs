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
//! payload's rows into them, and `collect` drives the run and journals it.

use anyhow::{ensure, Context, Result};
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
    /// State applied to targets that carry none. Only unambiguous for a single-state batch.
    pub states: Vec<String>,
}

/// One athlete to read, with the state that disambiguates its school.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub athlete_id: u64,
    pub state: Option<String>,
}

/// Parse the operator's athlete registry.
///
/// Lines are `athlete_id` or `athlete_id,ST`; a single `--states` value fills in the state for
/// targets that name none, and two or more are refused as ambiguous rather than guessed between.
pub fn parse_targets(body: &str, default_states: &[String]) -> Result<Vec<Target>> {
    ensure!(
        default_states.len() <= 1,
        "a registry without a per-line state needs exactly one --states value, got {} ({})",
        default_states.len(),
        default_states.join(",")
    );
    let fallback = default_states
        .first()
        .map(|state| state.trim().to_ascii_uppercase())
        .filter(|state| !state.is_empty());
    let mut targets = Vec::new();
    let mut seen = HashSet::new();
    for (number, line) in body.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split([',', '\t', ' ']).filter(|part| !part.is_empty());
        let id_text = parts.next().unwrap_or_default();
        let athlete_id: u64 = id_text.parse().with_context(|| {
            format!(
                "registry line {}: `{id_text}` is not an athlete id",
                number + 1
            )
        })?;
        let state = parts
            .next()
            .map(|state| state.trim().to_ascii_uppercase())
            .or_else(|| fallback.clone());
        ensure!(
            parts.next().is_none(),
            "registry line {}: expected `athlete_id[,ST]`, got `{line}`",
            number + 1
        );
        if let Some(state) = &state {
            ensure!(
                state.len() == 2 && state.chars().all(|c| c.is_ascii_alphabetic()),
                "registry line {}: `{state}` is not a two-letter state code",
                number + 1
            );
        }
        // A registry may legitimately repeat an athlete across state files; read it once.
        if seen.insert(athlete_id) {
            targets.push(Target { athlete_id, state });
        }
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
