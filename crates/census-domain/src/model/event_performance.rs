use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalEvent {
    pub id: EventId,
    pub meet: MeetId,
    pub kind: EventKind,
    pub gender: Gender,
    #[serde(default)]
    pub specification: EventSpecification,
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
        identity: EventIdentity<'_>,
        specification: EventSpecification,
    ) -> Result<Self, EventIdentityError> {
        let id = identity.mint(&specification)?;
        Ok(Self {
            id,
            meet: identity.meet.clone(),
            kind: identity.kind,
            gender: identity.gender,
            specification,
            division: identity.division.map(str::to_string),
            round: identity.round.map(str::to_string),
            source_labels: Vec::new(),
            evidence: Vec::new(),
            retained_conflicts: Vec::new(),
        })
    }

    pub fn resolved_source_kind(&self) -> Option<EventKind> {
        let EventKind::Unmapped { label } = &self.kind else {
            return None;
        };
        let resolved = EventKind::from_source_label(label);
        if matches!(resolved, EventKind::Unmapped { .. })
            || !self.retained_conflicts.is_empty()
            || !self
                .source_labels
                .iter()
                .any(|source| source.label == *label)
        {
            return None;
        }
        self.source_bound_kind(&resolved).then_some(resolved)
    }

    pub fn source_bound_kind(&self, kind: &EventKind) -> bool {
        !matches!(kind, EventKind::Unmapped { .. })
            && !self.source_labels.is_empty()
            && self.source_labels.iter().all(|source| {
                EventKind::from_source_label(&source.label) == *kind
                    && self.evidence.iter().any(|evidence| {
                        evidence.method == EvidenceMethod::Parsed
                            && evidence.source == source.source
                            && evidence
                                .source
                                .url
                                .as_ref()
                                .is_some_and(|url| !url.is_empty())
                    })
            })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mark {
    TimeSeconds(ExactSeconds),
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

fn mark_kind_compatible(kind: &EventKind, mark: &Mark) -> bool {
    let is_time_event = !kind.is_field() && !kind.is_relay();
    let is_time_mark = matches!(mark, Mark::TimeSeconds(_));
    let is_field_mark = matches!(mark, Mark::DistanceMetres(_) | Mark::FieldImperial { .. });
    let is_raw = matches!(mark, Mark::Raw(_));
    match kind {
        EventKind::Pentathlon | EventKind::Heptathlon | EventKind::Decathlon => {
            matches!(mark, Mark::Points(_)) || is_raw
        }
        EventKind::Relay4x100
        | EventKind::Relay4x200
        | EventKind::Relay4x400
        | EventKind::Relay4x800
        | EventKind::SprintMedley
        | EventKind::DistanceMedley => is_time_mark || is_raw,
        _ if kind.is_field() => is_field_mark || is_raw,
        _ if is_time_event => is_time_mark || is_raw,
        _ => true,
    }
}

impl CanonicalPerformance {
    pub fn new(
        identity: PerformanceIdentity<'_>,
        result: PerformanceResult<'_>,
    ) -> Result<Self, PerformanceError> {
        identity.validate()?;
        Ok(Self::from_identity(identity, result))
    }

    fn from_identity(identity: PerformanceIdentity<'_>, result: PerformanceResult<'_>) -> Self {
        Self {
            id: identity.mint(),
            athlete: identity.athlete.clone(),
            team: result.team.clone(),
            event: identity.event.clone(),
            meet: identity.meet.clone(),
            date: identity.date.to_owned(),
            mark: result.mark,
            wind_mps: result.wind_mps,
            place: result.place,
            heat: None,
            round: None,
            timing: None,
            observed_grade: None,
            evidence: Vec::new(),
            source_key: identity.source_key.to_owned(),
            source_athlete: None,
            retained_conflicts: Vec::new(),
        }
    }

    pub fn mark_compatible(&self, kind: &EventKind) -> bool {
        mark_kind_compatible(kind, &self.mark)
    }

    pub fn mint(
        athlete: &AthleteId,
        meet: &MeetId,
        event: &EventId,
        date: &str,
        source_key: &str,
    ) -> PerformanceId {
        Id::mint(
            "perf",
            &[
                athlete.as_str(),
                meet.as_str(),
                event.as_str(),
                date,
                source_key,
            ],
        )
    }
}
