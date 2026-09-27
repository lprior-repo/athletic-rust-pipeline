use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalEvent {
    pub id: EventId,
    pub meet: MeetId,
    pub kind: EventKind,
    pub gender: Gender,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub division: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub round: Option<String>,
    pub source_labels: Vec<SourceEventLabel>,
    pub evidence: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_conflicts: Vec<RetainedConflict>,
}

impl CanonicalEvent {
    pub fn new(
        meet: &MeetId,
        kind: EventKind,
        gender: Gender,
        division: Option<&str>,
        round: Option<&str>,
    ) -> Self {
        let key = kind.stable_key();
        let id = Id::mint(
            "evt",
            &[
                meet.as_str(),
                key.as_ref(),
                match gender {
                    Gender::Boys => "m",
                    Gender::Girls => "f",
                    _ => "u",
                },
                division.unwrap_or(""),
                round.unwrap_or(""),
            ],
        );
        Self {
            id,
            meet: meet.clone(),
            kind,
            gender,
            division: division.map(str::to_string),
            round: round.map(str::to_string),
            source_labels: Vec::new(),
            evidence: Vec::new(),
            retained_conflicts: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mark {
    TimeSeconds(CentiSeconds),
    DistanceMetres(CentiMetres),
    FieldImperial {
        feet_mark: String,
        metres: CentiMetres,
    },
    Points(CentiPoints),
    Raw(String),
}

impl Mark {
    pub fn raw(&self) -> &str {
        match self {
            Mark::Raw(value) => value,
            Mark::TimeSeconds(_) => "time",
            Mark::DistanceMetres(_) => "distance",
            Mark::FieldImperial { .. } => "field",
            Mark::Points(_) => "points",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalPerformance {
    pub id: PerformanceId,
    pub athlete: AthleteId,
    pub team: TeamId,
    pub event: EventId,
    pub meet: MeetId,
    pub date: String,
    pub mark: Mark,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind_mps: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heat: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub round: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<TimingMethod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_grade: Option<Grade>,
    pub evidence: Vec<Evidence>,
    pub source_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_athlete: Option<SourceIdentity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_conflicts: Vec<RetainedConflict>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimingMethod {
    Fat,
    Hand,
    Unknown,
}

impl TimingMethod {
    pub const fn stable_key(self) -> &'static str {
        match self {
            Self::Fat => "Fat",
            Self::Hand => "Hand",
            Self::Unknown => "Unknown",
        }
    }
}

impl CanonicalPerformance {
    pub fn mint(
        athlete: &AthleteId,
        meet: &MeetId,
        kind: &EventKind,
        date: &str,
        source_key: &str,
    ) -> PerformanceId {
        let kind_key = kind.stable_key();
        Id::mint(
            "perf",
            &[
                athlete.as_str(),
                meet.as_str(),
                kind_key.as_ref(),
                date,
                source_key,
            ],
        )
    }
}
