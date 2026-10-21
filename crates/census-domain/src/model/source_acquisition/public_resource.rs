mod accessors;
mod codec;
mod origin;
mod target;
mod wire;

use super::identity::validate_meet_id;
use super::CaptureDomainError;
use crate::model::{SchoolYear, Sport};
use serde::{Deserialize, Serialize};
use wire::Wire;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PublicSourceResourceV1(Wire);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicSourceResourceKindV1 {
    MileSplitTeamDirectory,
    MileSplitAthleteProfile,
    MileSplitTeamRoster,
    MileSplitMeetTeam,
    MileSplitRawResults,
    MileSplitTeamMeetHistory,
    SchoolCalendar,
    Robots,
}

impl PublicSourceResourceV1 {
    pub const fn milesplit_team_directory() -> Self {
        Self(Wire::MileSplitTeamDirectory {})
    }

    pub fn milesplit_athlete_profile(athlete_id: &str) -> Result<Self, CaptureDomainError> {
        validate_meet_id(athlete_id)?;
        Ok(Self(Wire::MileSplitAthleteProfile {
            athlete_id: copy(athlete_id)?,
        }))
    }

    pub fn milesplit_team_roster(team_id: &str) -> Result<Self, CaptureDomainError> {
        validate_meet_id(team_id)?;
        Ok(Self(Wire::MileSplitTeamRoster {
            team_id: copy(team_id)?,
        }))
    }

    pub fn milesplit_meet_team(meet_id: &str, team_id: &str) -> Result<Self, CaptureDomainError> {
        validate_meet_id(meet_id)?;
        validate_meet_id(team_id)?;
        Ok(Self(Wire::MileSplitMeetTeam {
            meet_id: copy(meet_id)?,
            team_id: copy(team_id)?,
        }))
    }

    pub fn milesplit_raw_results(
        meet_id: &str,
        result_id: &str,
    ) -> Result<Self, CaptureDomainError> {
        validate_meet_id(meet_id)?;
        validate_meet_id(result_id)?;
        Ok(Self(Wire::MileSplitRawResults {
            meet_id: copy(meet_id)?,
            result_id: copy(result_id)?,
        }))
    }

    pub fn milesplit_team_meet_history(team_id: &str) -> Result<Self, CaptureDomainError> {
        validate_meet_id(team_id)?;
        Ok(Self(Wire::MileSplitTeamMeetHistory {
            team_id: copy(team_id)?,
        }))
    }

    pub fn school_calendar(
        provider_school_id: &str,
        origin: &str,
        sport: Sport,
        school_year: SchoolYear,
    ) -> Result<Self, CaptureDomainError> {
        wire::validate_calendar(provider_school_id, origin, sport)?;
        Ok(Self(Wire::SchoolCalendar {
            provider_school_id: copy(provider_school_id)?,
            origin: copy(origin)?,
            sport,
            school_year,
        }))
    }

    pub fn robots(origin: &str) -> Result<Self, CaptureDomainError> {
        origin::validate(origin)?;
        Ok(Self(Wire::Robots {
            origin: copy(origin)?,
        }))
    }
}

fn copy(value: &str) -> Result<String, CaptureDomainError> {
    super::scalar::try_text(value).map_err(|_| CaptureDomainError::Invalid {
        field: "public resource allocation",
    })
}

impl TryFrom<Wire> for PublicSourceResourceV1 {
    type Error = CaptureDomainError;

    fn try_from(wire: Wire) -> Result<Self, Self::Error> {
        wire.validate()?;
        Ok(Self(wire))
    }
}

impl<'de> Deserialize<'de> for PublicSourceResourceV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::try_from(codec::decode(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
