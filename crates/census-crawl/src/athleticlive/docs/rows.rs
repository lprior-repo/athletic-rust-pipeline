//! The row-level wire types: one result row, its athlete, and the team that athlete competed for.
//!
//! Part of the reader contract in [`super`]: the event-document and event-summary payloads share
//! these shapes, and `docs::marks` holds the mark rules a row's two published channels resolve by.

use serde::Deserialize;
use serde_json::Value;

use super::marks::row_mark;
use super::value_u64;
use census_domain::model::{EventKind, Mark};

/// The event's series/division block (for example the Michigan Indoor Track Series, `MITS`).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DocDivision {
    #[serde(default)]
    pub n: Option<String>,
}

/// The team a result row's athlete competed for.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DocTeam {
    /// AthleticLIVE team id (the same id space the athlete index publishes).
    #[serde(default, rename = "i")]
    pub timer_team_id: Option<Value>,
    #[serde(default, rename = "n")]
    pub name: Option<String>,
    #[serde(default, rename = "f")]
    pub short_name: Option<String>,
    /// Athletic.net team id.
    #[serde(default, rename = "ani")]
    pub an_team_id: Option<Value>,
}

impl DocTeam {
    /// The school name as published: long name preferred, then short name.
    pub fn school_name(&self) -> Option<&str> {
        self.name
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .or_else(|| {
                self.short_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
            })
    }
}

/// The athlete a result row describes.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DocAthlete {
    #[serde(default, rename = "n")]
    pub name: Option<String>,
    /// Grade token: `"9".."12"`, `FR|SO|JR|SR`, or a below-high-school grade on open meets.
    #[serde(default, rename = "y")]
    pub grade: Option<Value>,
    /// Athletic.net athlete id.
    #[serde(default, rename = "ani")]
    pub an_athlete_id: Option<Value>,
    /// The competitor's own gender token (`Male`, `Female`).
    #[serde(default, rename = "g")]
    pub gender: Option<String>,
    #[serde(default, rename = "t")]
    pub team: Option<DocTeam>,
}

impl DocAthlete {
    /// The competitor's own gender token (`Male`, `Female`), which the event's side falls back to.
    pub fn gender_token(&self) -> Option<&str> {
        self.gender
            .as_deref()
            .map(str::trim)
            .filter(|token| !token.is_empty())
    }
}

/// One published result row.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DocRow {
    /// Place as published (`"1"`, or `"--"` when unplaced).
    #[serde(default, rename = "p")]
    pub place: Option<Value>,
    /// The mark as published (`4:40.80`, `5-02.00`, `NH`).
    #[serde(default, rename = "m")]
    pub mark: Option<String>,
    /// The mark in the platform's integer channel: milliseconds, or micrometres for field marks.
    #[serde(default, rename = "im")]
    pub mark_int: Option<Value>,
    #[serde(default, rename = "w")]
    pub wind: Option<Value>,
    #[serde(default, rename = "hn")]
    pub heat: Option<Value>,
    /// The published seed mark.
    #[serde(default, rename = "s")]
    pub seed: Option<String>,
    /// The platform's cross-country split list: one entry per split, each the payload's own copy of
    /// the row carrying that segment's times. Counted as a channel; no observation is minted here.
    #[serde(default, rename = "irs")]
    pub splits: Vec<Value>,
    #[serde(default, rename = "a")]
    pub athlete: Option<DocAthlete>,
}

impl DocRow {
    /// The place, when the row is placed.
    pub fn place(&self) -> Option<u16> {
        let place = self.place.as_ref().and_then(value_u64)?;
        u16::try_from(place).ok().filter(|p| *p > 0)
    }

    /// Wind in metres per second, when the row publishes it.
    pub fn wind_mps(&self) -> Option<f64> {
        match self.wind.as_ref()? {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        }
    }

    /// The heat number, when the row publishes one.
    pub fn heat_number(&self) -> Option<u64> {
        self.heat.as_ref().and_then(value_u64)
    }

    /// The canonical mark in the units `kind` implies, or `None` when the row published none
    /// (`NH` publishes `im: 0`).
    pub fn canonical_mark(&self, kind: &EventKind) -> Option<Mark> {
        row_mark(kind, self.mark.as_deref(), self.mark_int.as_ref())
    }
}
