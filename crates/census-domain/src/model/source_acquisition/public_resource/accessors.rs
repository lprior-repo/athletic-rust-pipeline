use super::{PublicSourceResourceKindV1, PublicSourceResourceV1, SchoolYear, Sport, Wire};

impl PublicSourceResourceV1 {
    pub fn kind(&self) -> PublicSourceResourceKindV1 {
        match self.0 {
            Wire::MileSplitTeamDirectory {} => PublicSourceResourceKindV1::MileSplitTeamDirectory,
            Wire::MileSplitAthleteProfile { .. } => {
                PublicSourceResourceKindV1::MileSplitAthleteProfile
            }
            Wire::MileSplitTeamRoster { .. } => PublicSourceResourceKindV1::MileSplitTeamRoster,
            Wire::MileSplitMeetTeam { .. } => PublicSourceResourceKindV1::MileSplitMeetTeam,
            Wire::MileSplitRawResults { .. } => PublicSourceResourceKindV1::MileSplitRawResults,
            Wire::MileSplitTeamMeetHistory { .. } => {
                PublicSourceResourceKindV1::MileSplitTeamMeetHistory
            }
            Wire::SchoolCalendar { .. } => PublicSourceResourceKindV1::SchoolCalendar,
            Wire::Robots { .. } => PublicSourceResourceKindV1::Robots,
        }
    }

    pub fn athlete_id(&self) -> Option<&str> {
        match &self.0 {
            Wire::MileSplitAthleteProfile { athlete_id } => Some(athlete_id),
            _ => None,
        }
    }

    pub fn team_id(&self) -> Option<&str> {
        match &self.0 {
            Wire::MileSplitTeamRoster { team_id }
            | Wire::MileSplitMeetTeam { team_id, .. }
            | Wire::MileSplitTeamMeetHistory { team_id } => Some(team_id),
            _ => None,
        }
    }

    pub fn meet_id(&self) -> Option<&str> {
        match &self.0 {
            Wire::MileSplitMeetTeam { meet_id, .. } | Wire::MileSplitRawResults { meet_id, .. } => {
                Some(meet_id)
            }
            _ => None,
        }
    }

    pub fn result_id(&self) -> Option<&str> {
        match &self.0 {
            Wire::MileSplitRawResults { result_id, .. } => Some(result_id),
            _ => None,
        }
    }

    pub fn provider_school_id(&self) -> Option<&str> {
        match &self.0 {
            Wire::SchoolCalendar {
                provider_school_id, ..
            } => Some(provider_school_id),
            _ => None,
        }
    }

    pub fn origin(&self) -> Option<&str> {
        match &self.0 {
            Wire::SchoolCalendar { origin, .. } | Wire::Robots { origin } => Some(origin),
            _ => None,
        }
    }

    pub fn sport(&self) -> Option<Sport> {
        match self.0 {
            Wire::SchoolCalendar { sport, .. } => Some(sport),
            _ => None,
        }
    }

    pub fn school_year(&self) -> Option<SchoolYear> {
        match self.0 {
            Wire::SchoolCalendar { school_year, .. } => Some(school_year),
            _ => None,
        }
    }
}
