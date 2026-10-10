use serde::{Deserialize, Serialize};

use super::{
    AthleteId, EventId, Evidence, Id, Mark, MeetId, RelayId, RetainedConflict, SourceIdentity,
    TeamId, TimingMethod,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayMember {
    pub order: u32,
    pub name_as_published: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub athlete: Option<AthleteId>,
}

impl RelayMember {
    pub fn new(order: u32, name_as_published: impl Into<String>) -> Self {
        Self {
            order,
            name_as_published: name_as_published.into(),
            athlete: None,
        }
    }

    pub fn with_athlete(mut self, athlete: AthleteId) -> Self {
        self.athlete = Some(athlete);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayResult {
    pub id: RelayId,
    pub team: TeamId,
    pub event: EventId,
    pub meet: MeetId,
    pub mark: Mark,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub round: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<TimingMethod>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<RelayMember>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_conflicts: Vec<RetainedConflict>,
    pub source_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_team: Option<SourceIdentity>,
}

impl RelayResult {
    pub fn new(
        team: &TeamId,
        event: &EventId,
        meet: &MeetId,
        mark: Mark,
        source_key: impl Into<String>,
    ) -> Self {
        Self {
            id: Self::mint(team, event, meet),
            team: team.clone(),
            event: event.clone(),
            meet: meet.clone(),
            mark,
            place: None,
            round: None,
            timing: None,
            members: Vec::new(),
            evidence: Vec::new(),
            retained_conflicts: Vec::new(),
            source_key: source_key.into(),
            source_team: None,
        }
    }

    pub fn mint(team: &TeamId, event: &EventId, meet: &MeetId) -> RelayId {
        Id::mint("relay", &[meet.as_str(), event.as_str(), team.as_str()])
    }

    pub fn with_place(mut self, place: Option<u16>) -> Self {
        self.place = place;
        self
    }

    pub fn with_round(mut self, round: Option<String>) -> Self {
        self.round = round;
        self
    }

    pub fn with_timing(mut self, timing: Option<TimingMethod>) -> Self {
        self.timing = timing;
        self
    }

    pub fn with_member(mut self, member: RelayMember) -> Self {
        self.members.push(member);
        self
    }

    pub fn with_evidence(mut self, evidence: Evidence) -> Self {
        self.evidence.push(evidence);
        self
    }

    pub fn with_source_team(mut self, source_team: SourceIdentity) -> Self {
        self.source_team = Some(source_team);
        self
    }
}
