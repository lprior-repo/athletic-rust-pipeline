use super::codec::{corrupt, FrozenCodecResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    Athletes,
    Schools,
    Teams,
    Coaches,
    CoachObservations,
    Events,
    Meets,
    Performances,
    ReviewCases,
    SourceAccess,
    Verdicts,
    IdentityDecisions,
    CoverageRows,
    CoachLanes,
}

impl StreamKind {
    pub const ALL: [StreamKind; 14] = [
        StreamKind::Athletes,
        StreamKind::Schools,
        StreamKind::Teams,
        StreamKind::Coaches,
        StreamKind::CoachObservations,
        StreamKind::Events,
        StreamKind::Meets,
        StreamKind::Performances,
        StreamKind::ReviewCases,
        StreamKind::SourceAccess,
        StreamKind::Verdicts,
        StreamKind::IdentityDecisions,
        StreamKind::CoverageRows,
        StreamKind::CoachLanes,
    ];

    pub const fn index(self) -> usize {
        match self {
            Self::Athletes => 0,
            Self::Schools => 1,
            Self::Teams => 2,
            Self::Coaches => 3,
            Self::CoachObservations => 4,
            Self::Events => 5,
            Self::Meets => 6,
            Self::Performances => 7,
            Self::ReviewCases => 8,
            Self::SourceAccess => 9,
            Self::Verdicts => 10,
            Self::IdentityDecisions => 11,
            Self::CoverageRows => 12,
            Self::CoachLanes => 13,
        }
    }

    pub const fn wire(self) -> &'static str {
        match self {
            Self::Athletes => "athletes",
            Self::Schools => "schools",
            Self::Teams => "teams",
            Self::Coaches => "coaches",
            Self::CoachObservations => "coach_observations",
            Self::Events => "events",
            Self::Meets => "meets",
            Self::Performances => "performances",
            Self::ReviewCases => "review_cases",
            Self::SourceAccess => "source_access",
            Self::Verdicts => "verdicts",
            Self::IdentityDecisions => "identity_decisions",
            Self::CoverageRows => "coverage_rows",
            Self::CoachLanes => "coach_lanes",
        }
    }

    pub fn from_wire(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.wire() == name)
    }

    pub const fn keyed_by_id(self) -> bool {
        matches!(self, Self::Schools | Self::Teams)
    }

    pub const fn unique_ids(self) -> bool {
        !matches!(self, Self::CoachObservations)
    }
}

pub fn check_identity_order(
    stream: StreamKind,
    previous: Option<&str>,
    current: &str,
) -> FrozenCodecResult<()> {
    let Some(previous) = previous else {
        return Ok(());
    };
    let ordered = if stream.unique_ids() {
        current > previous
    } else {
        current >= previous
    };
    if ordered {
        return Ok(());
    }
    Err(corrupt(
        "frozen stream order",
        &format!("{} moved from {previous} back to {current}", stream.wire()),
    ))
}
