
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AthleteHit {
    #[serde(default)]
    pub i: Option<Value>,
    #[serde(default)]
    pub n: Option<String>,
    #[serde(default)]
    pub y: Option<Value>,
    #[serde(default)]
    pub g: Option<String>,
    #[serde(default)]
    pub mi: Option<Value>,
    #[serde(default)]
    pub ani: Option<Value>,
    #[serde(default)]
    pub t: Option<HitTeam>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HitTeam {
    #[serde(default)]
    pub i: Option<Value>,
    #[serde(default)]
    pub n: Option<String>,
    #[serde(default)]
    pub f: Option<String>,
    #[serde(default)]
    pub ab: Option<String>,
    #[serde(default)]
    pub ani: Option<Value>,
    #[serde(default)]
    pub xc: Option<Value>,
}

impl HitTeam {
    pub fn school_name(&self) -> Option<&str> {
        self.n
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| self.f.as_deref().filter(|v| !v.trim().is_empty()))
    }
}

fn as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().parse::<u64>().ok(),
        _ => None,
    }
}

impl AthleteHit {
    pub fn athletic_net_athlete_id(&self) -> Option<u64> {
        self.ani.as_ref().and_then(as_u64)
    }

    pub fn athleticlive_row_id(&self) -> Option<u64> {
        self.i.as_ref().and_then(as_u64)
    }

    pub fn meet_id(&self) -> Option<u64> {
        self.mi.as_ref().and_then(as_u64)
    }
}

impl HitTeam {
    pub fn athletic_net_team_id(&self) -> Option<u64> {
        self.ani.as_ref().and_then(as_u64)
    }

    pub fn athleticlive_team_id(&self) -> Option<u64> {
        self.i.as_ref().and_then(as_u64)
    }

    pub fn is_cross_country(&self) -> bool {
        match &self.xc {
            Some(Value::Number(n)) => n.as_i64().map(|v| v != 0).unwrap_or(false),
            Some(Value::Bool(b)) => *b,
            Some(Value::String(s)) => s.trim() != "0" && !s.trim().is_empty(),
            _ => false,
        }
    }
}
