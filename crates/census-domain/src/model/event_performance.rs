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
    /// Canonical-id collisions this row's merge retained: another natural key minted this id, so the
    /// row below is the one that survived and the other subject's facts were not absorbed. Empty on
    /// every row whose fields still state the id they minted, which is every row until one collides.
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
        let id = Id::mint(
            "evt",
            &[
                meet.as_str(),
                &format!("{kind:?}"),
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

/// A mark's unit context. Track marks are times (or points for combined events), field marks are
/// distances/heights in metric or imperial notation, exactly as published by the source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mark {
    /// Seconds (e.g. `10.94`, `4:41.23` already converted to 281.23).
    TimeSeconds(f64),
    /// Metres, converted from the published imperial/metric value.
    DistanceMetres(f64),
    /// A field mark preserved in the source's own notation (e.g. `5' 4"`, `42-06.5`).
    FieldImperial { feet_mark: String, metres: f64 },
    /// Combined-event or team points.
    Points(f64),
    /// Published verbatim, not yet parsed.
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
    /// The athlete's grade at the moment of this performance, when the source publishes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_grade: Option<Grade>,
    pub evidence: Vec<Evidence>,
    /// Provider-local result key, used for idempotent upserts.
    pub source_key: String,
    /// Canonical-id collisions this row's merge retained: another natural key minted this id, so the
    /// row below is the one that survived and the other subject's facts were not absorbed. Empty on
    /// every row whose fields still state the id they minted, which is every row until one collides.
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
