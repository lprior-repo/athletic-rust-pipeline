use super::main::PageParseError;
use crate::{
    domain::{identity::AthleteId, name::CanonicalName},
    runtime::rankings::types::{PageObservation, VerifiedRelayMember},
};
use serde_json::{Map, Value};

pub(super) fn parse_relay_roster(
    raw: &Value,
    observation: &mut PageObservation,
) -> Result<(), PageParseError> {
    let teams = optional_teams(raw)?;
    let total_teams = teams.map_or(0, Map::len);
    if total_teams > 1_024 {
        return Err(PageParseError::RelayTeamLimitExceeded);
    }
    observation.total_relay_teams =
        u64::try_from(total_teams).map_err(|_| PageParseError::CounterOverflow)?;
    let groups = raw
        .get("groupedRankings")
        .and_then(Value::as_array)
        .ok_or(PageParseError::MissingGroupedRankings)?;
    groups
        .iter()
        .enumerate()
        .try_fold(0_usize, |flattened, (group_index, group)| {
            let rows = group
                .as_array()
                .ok_or(PageParseError::NonArrayGroup { index: group_index })?;
            rows.iter()
                .enumerate()
                .try_fold(flattened, |flattened, (row_index, row)| {
                    project_row(teams, row, group_index, row_index, flattened, observation)?;
                    flattened
                        .checked_add(1)
                        .ok_or(PageParseError::CounterOverflow)
                })
        })
        .map(|_| ())
}

fn optional_teams(raw: &Value) -> Result<Option<&Map<String, Value>>, PageParseError> {
    match raw.get("relayTeams") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(teams)) => Ok(Some(teams)),
        Some(_) => Err(PageParseError::WrongRelayTeamsType),
    }
}

fn members(team: &Value, result_id: u64, team_id: u64) -> Result<&[Value], PageParseError> {
    let roster_id = team
        .get("IDResult")
        .and_then(Value::as_u64)
        .filter(|id| *id != 0)
        .ok_or(PageParseError::WrongRelayTeamId)?;
    let roster_team = team
        .get("RelayTeamID")
        .and_then(Value::as_u64)
        .filter(|id| *id != 0)
        .ok_or(PageParseError::WrongRelayTeamId)?;
    if roster_id != result_id || roster_team != team_id {
        return Err(PageParseError::WrongRelayJoin {
            roster_id_result: roster_id,
            row_id_result: result_id,
            roster_relay_team_id: roster_team,
            row_athlete_id: team_id,
        });
    }
    match team.get("Members") {
        None | Some(Value::Null) => Ok(&[]),
        Some(Value::Array(values)) => Ok(values),
        Some(_) => Err(PageParseError::WrongMembersType),
    }
}

fn project_row(
    teams: Option<&Map<String, Value>>,
    row: &Value,
    group_index: usize,
    row_index: usize,
    flattened: usize,
    observation: &mut PageObservation,
) -> Result<(), PageParseError> {
    let result_id = row
        .get("IDResult")
        .and_then(Value::as_u64)
        .filter(|id| *id != 0)
        .ok_or(PageParseError::MissingRowIdResult)?;
    let roster = teams.and_then(|entries| entries.get(&result_id.to_string()));
    let source = observation
        .source_rows
        .get_mut(flattened)
        .ok_or(PageParseError::MissingRowNum)?;
    let Some(roster) = roster else {
        source.roster_present = Some(false);
        observation.rows_missing_roster = observation
            .rows_missing_roster
            .checked_add(1)
            .ok_or(PageParseError::CounterOverflow)?;
        return Ok(());
    };
    let team_id = row
        .get("AthleteID")
        .and_then(Value::as_u64)
        .filter(|id| *id != 0);
    // A blurred row keeps its roster entry in the payload but not its identity:
    // this session receives `AthleteID: 0` for masked rows while `relayTeams`
    // still carries their `IDResult` key, so the join cannot be verified. The
    // row stays counted as an unresolved roster and the page still parses,
    // mirroring the individual path's handling of anonymous rows.
    let Some(team_id) = team_id else {
        source.roster_present = Some(false);
        observation.rows_missing_roster = observation
            .rows_missing_roster
            .checked_add(1)
            .ok_or(PageParseError::CounterOverflow)?;
        return Ok(());
    };
    let members = members(roster, result_id, team_id)?;
    source.roster_present = Some(true);
    observation.rows_with_roster = observation
        .rows_with_roster
        .checked_add(1)
        .ok_or(PageParseError::CounterOverflow)?;
    members.iter().enumerate().try_for_each(|(index, member)| {
        project_member(
            member,
            index,
            result_id,
            team_id,
            group_index,
            row_index,
            observation,
        )
    })
}

fn project_member(
    member: &Value,
    member_index: usize,
    result_id: u64,
    team_id: u64,
    group_index: usize,
    row_index: usize,
    observation: &mut PageObservation,
) -> Result<(), PageParseError> {
    if member.get("GradeID").and_then(Value::as_u64) != Some(11) {
        return Ok(());
    }
    let identity = member
        .get("IDAthlete")
        .and_then(Value::as_u64)
        .zip(member.get("AthleteName").and_then(Value::as_str));
    let parsed = identity
        .and_then(|(id, name)| AthleteId::new(id).ok().zip(CanonicalName::parse(name).ok()));
    let Some((athlete_id, name)) = parsed else {
        observation.unresolved_member_identities = observation
            .unresolved_member_identities
            .checked_add(1)
            .ok_or(PageParseError::CounterOverflow)?;
        return Ok(());
    };
    if observation.verified_relay_members.len() >= super::main::MAX_PAGE_CANDIDATES {
        return Err(PageParseError::CandidateLimitExceeded);
    }
    observation
        .verified_relay_members
        .push(VerifiedRelayMember {
            athlete_id,
            name,
            handle: member
                .get("Handle")
                .and_then(Value::as_str)
                .map_or_else(String::new, str::to_owned),
            photo_url: member
                .get("PhotoUrl")
                .and_then(Value::as_str)
                .map(str::to_owned),
            grade_id: 11,
            id_result: result_id,
            roster_relay_team_id: team_id,
            row_athlete_id: team_id,
            row_locator: Some(format!("/groupedRankings/{group_index}/{row_index}")),
            member_locator: Some(member_index),
        });
    Ok(())
}
