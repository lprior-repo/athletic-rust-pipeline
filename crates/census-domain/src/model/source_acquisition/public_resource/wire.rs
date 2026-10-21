use super::super::identity::validate_meet_id;
use super::{origin, CaptureDomainError, SchoolYear, Sport};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Wire {
    MileSplitTeamDirectory {},
    MileSplitAthleteProfile {
        athlete_id: String,
    },
    MileSplitTeamRoster {
        team_id: String,
    },
    MileSplitMeetTeam {
        meet_id: String,
        team_id: String,
    },
    MileSplitRawResults {
        meet_id: String,
        result_id: String,
    },
    MileSplitTeamMeetHistory {
        team_id: String,
    },
    SchoolCalendar {
        provider_school_id: String,
        origin: String,
        sport: Sport,
        school_year: SchoolYear,
    },
    Robots {
        origin: String,
    },
}

impl Wire {
    pub(super) fn validate(&self) -> Result<(), CaptureDomainError> {
        match self {
            Self::MileSplitTeamDirectory {} => Ok(()),
            Self::MileSplitAthleteProfile { athlete_id } => validate_meet_id(athlete_id),
            Self::MileSplitTeamRoster { team_id } | Self::MileSplitTeamMeetHistory { team_id } => {
                validate_meet_id(team_id)
            }
            Self::MileSplitMeetTeam { meet_id, team_id } => {
                validate_meet_id(meet_id)?;
                validate_meet_id(team_id)
            }
            Self::MileSplitRawResults { meet_id, result_id } => {
                validate_meet_id(meet_id)?;
                validate_meet_id(result_id)
            }
            Self::SchoolCalendar {
                provider_school_id,
                origin: value,
                sport,
                ..
            } => validate_calendar(provider_school_id, value, *sport),
            Self::Robots { origin: value } => origin::validate(value),
        }
    }
}

pub(super) fn validate_calendar(
    provider_school_id: &str,
    value: &str,
    sport: Sport,
) -> Result<(), CaptureDomainError> {
    validate_meet_id(provider_school_id)?;
    if sport == Sport::Unknown {
        return Err(CaptureDomainError::Invalid {
            field: "calendar sport",
        });
    }
    origin::validate(value)
}
