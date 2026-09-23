//! Wire reader for the live-standings fallback (`liveRunStandings/<runId>.json`).
//!
//! ```text
//! {"10exm2":{"i":"1543","cm":"1543","n":"Jaydyn Velek","fn":"Jaydyn","l":"Velek","g":"M",
//!            "y":"SR","tn":"Jamestown","ti":"2eW0TN","p":98,"rtm":"18:23.114","m":"18:23.2",
//!            "ani":20010099,"anli":42660317,"gap":"2:55.4","cc":"pc-dn","iv":"1.0",
//!            "sp":{"0":{..,"sp":"5:26.0","cs":"5:26.0"},"1":{..},"split_final":{..}}}}
//! ```
//!
//! The map key is the platform's push id for the row, `ti` is a short team key, and every `sp`
//! entry is a full copy of the row carrying that split's `sp`/`cs`. `split_final` is the finish, not
//! a split, and is not counted as one.
//!
//! The standings payload publishes no integer mark channel: times take the raw column (`rtm`,
//! `18:23.114` for the published `18:23.2`), field marks take the published column (`m`), and each
//! falls back to the other. `anli` is read and counted but never minted into an identity — its
//! athlete-level semantics are `[I]` in `[sources/state-assoc-plains]`, not measured.

use serde::Deserialize;
use serde_json::Value;

use crate::hytek::{parse_field_mark, parse_time};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{EventKind, Mark};

use super::docs::value_u64;

/// The key that marks the finishing split rather than an intermediate one.
const FINISH_SPLIT: &str = "split_final";

/// One row of a live race: the finishing order as it was published.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct StandingRow {
    #[serde(default, rename = "n")]
    pub name: Option<String>,
    /// Gender token (`M`/`F`).
    #[serde(default, rename = "g")]
    pub gender: Option<String>,
    /// Grade token: `SR|JR|SO|FR` on the captured state final, or a numeric open-meet grade.
    #[serde(default, rename = "y")]
    pub grade: Option<Value>,
    /// Team name as published (`Jamestown`).
    #[serde(default, rename = "tn")]
    pub team_name: Option<String>,
    /// The platform's short team key for this run.
    #[serde(default, rename = "ti")]
    pub team_key: Option<String>,
    #[serde(default, rename = "p")]
    pub place: Option<Value>,
    /// The mark as published, rounded for display.
    #[serde(default, rename = "m")]
    pub mark: Option<String>,
    /// The raw timing channel (chip/real time) the display column rounds.
    #[serde(default, rename = "rtm")]
    pub raw_time: Option<String>,
    /// Athletic.net athlete id.
    #[serde(default, rename = "ani")]
    pub an_athlete_id: Option<Value>,
    /// The second Athletic.net-derived id channel; read, counted, not minted.
    #[serde(default, rename = "anli")]
    pub an_legacy_id: Option<Value>,
    /// Per-split copies of the row, keyed `"0"`, `"1"`, … and `split_final`.
    #[serde(default, rename = "sp")]
    pub splits: std::collections::BTreeMap<String, Value>,
}

impl StandingRow {
    pub fn name(&self) -> Option<&str> {
        self.name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
    }

    pub fn school_name(&self) -> Option<&str> {
        self.team_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
    }

    pub fn an_athlete_id(&self) -> Option<u64> {
        self.an_athlete_id.as_ref().and_then(value_u64)
    }

    pub fn has_legacy_id(&self) -> bool {
        self.an_legacy_id.as_ref().and_then(value_u64).is_some()
    }

    pub fn has_team_key(&self) -> bool {
        self.team_key
            .as_deref()
            .is_some_and(|key| !key.trim().is_empty())
    }

    pub fn place(&self) -> Option<u16> {
        let place = self.place.as_ref().and_then(value_u64)?;
        u16::try_from(place).ok().filter(|place| *place > 0)
    }

    /// Intermediate splits, finish marker excluded.
    pub fn split_count(&self) -> usize {
        self.splits
            .keys()
            .filter(|key| key.as_str() != FINISH_SPLIT)
            .count()
    }

    /// The canonical mark: the raw timing channel for times, the published column for field marks.
    pub fn canonical_mark(&self, kind: &EventKind) -> Option<Mark> {
        let (first, second) = if kind.is_field() {
            (self.mark.as_deref(), self.raw_time.as_deref())
        } else {
            (self.raw_time.as_deref(), self.mark.as_deref())
        };
        let token = first
            .into_iter()
            .chain(second)
            .map(str::trim)
            .find(|token| !token.is_empty())?;
        if kind.is_field() {
            return parse_field_mark(token);
        }
        parse_time(token).map(Mark::TimeSeconds)
    }
}

/// Read a live-standings document, ordered by the payload's own keys.
pub fn parse_standings(url: &str, body: &str) -> CrawlResult<Vec<(String, StandingRow)>> {
    let rows: std::collections::BTreeMap<String, StandingRow> = serde_json::from_str(body)
        .map_err(|source| CrawlError::Decode {
            url: url.to_string(),
            source,
        })?;
    Ok(rows.into_iter().collect())
}
