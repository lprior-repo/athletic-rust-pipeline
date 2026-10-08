use serde::Deserialize;
use serde_json::Value;

use super::marks::{row_mark, MarkError};
use super::value_u64;
use census_domain::model::{EventKind, Mark};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DocDivision {
    #[serde(default)]
    pub n: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DocTeam {
    #[serde(default, rename = "i")]
    pub timer_team_id: Option<Value>,
    #[serde(default, rename = "n")]
    pub name: Option<String>,
    #[serde(default, rename = "f")]
    pub short_name: Option<String>,
    #[serde(default, rename = "ani")]
    pub an_team_id: Option<Value>,
}

impl DocTeam {
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

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DocAthlete {
    #[serde(default, rename = "n")]
    pub name: Option<String>,
    #[serde(default, rename = "y")]
    pub grade: Option<Value>,
    #[serde(default, rename = "ani")]
    pub an_athlete_id: Option<Value>,
    #[serde(default, rename = "g")]
    pub gender: Option<String>,
    #[serde(default, rename = "t")]
    pub team: Option<DocTeam>,
}

impl DocAthlete {
    pub fn gender_token(&self) -> Option<&str> {
        self.gender
            .as_deref()
            .map(str::trim)
            .filter(|token| !token.is_empty())
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DocRow {
    #[serde(default, rename = "p")]
    pub place: Option<Value>,
    #[serde(default, rename = "m")]
    pub mark: Option<String>,
    #[serde(default, rename = "im")]
    pub mark_int: Option<Value>,
    #[serde(default, rename = "vm")]
    pub validity: Option<Value>,
    #[serde(default, rename = "w")]
    pub wind: Option<Value>,
    #[serde(default, rename = "hn")]
    pub heat: Option<Value>,
    #[serde(default, rename = "s")]
    pub seed: Option<String>,
    #[serde(default, rename = "irs")]
    pub splits: Vec<Value>,
    #[serde(default, rename = "a")]
    pub athlete: Option<DocAthlete>,
}

impl DocRow {
    pub fn place(&self) -> Option<u16> {
        let place = self.place.as_ref().and_then(value_u64)?;
        u16::try_from(place).ok().filter(|p| *p > 0)
    }

    pub fn wind_mps(&self) -> Option<f64> {
        match self.wind.as_ref()? {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        }
    }

    pub fn heat_number(&self) -> Option<u64> {
        self.heat.as_ref().and_then(value_u64)
    }

    pub fn canonical_mark(&self, kind: &EventKind) -> Result<Option<Mark>, MarkError> {
        row_mark(
            kind,
            self.mark.as_deref(),
            self.mark_int.as_ref(),
            self.validity.as_ref(),
        )
    }
}
