use super::error::PageParseError;
use super::MAX_PAGE_CANDIDATES;
use crate::domain::identity::AthleteId;
use crate::domain::name::CanonicalName;
use crate::runtime::rankings::types::{IndividualCandidate, PageObservation};

/// The source publishes `AthleteID: 0` for rows it keeps anonymous; only a
/// positive id is an identity signal.
pub(super) fn published_athlete_id(row: &serde_json::Value) -> Option<u64> {
    row.get("AthleteID")
        .and_then(|value| value.as_u64())
        .filter(|athlete_id| *athlete_id != 0)
}

pub(super) fn parse_individual_candidate(
    row: &serde_json::Value,
    group_index: usize,
    row_index: usize,
    observation: &mut PageObservation,
    (candidate_index, unresolved, candidate_count): (u64, u64, u64),
) -> Result<(u64, u64, u64), PageParseError> {
    let identity =
        published_athlete_id(row).zip(row.get("AthleteName").and_then(|value| value.as_str()));
    let parsed = identity.and_then(|(id, name)| {
        CanonicalName::parse(name)
            .ok()
            .zip(AthleteId::new(id).ok())
            .map(|(name, athlete_id)| (athlete_id, name))
    });
    let Some((athlete_id, name)) = parsed else {
        return Ok((
            candidate_index,
            unresolved
                .checked_add(1)
                .ok_or(PageParseError::CounterOverflow)?,
            candidate_count,
        ));
    };
    if observation.grade_11_candidates_list.len() >= MAX_PAGE_CANDIDATES {
        return Err(PageParseError::CandidateLimitExceeded);
    }
    let record_index = candidate_index;
    let next_index = candidate_index
        .checked_add(1)
        .ok_or(PageParseError::CounterOverflow)?;
    let next_count = candidate_count
        .checked_add(1)
        .ok_or(PageParseError::CounterOverflow)?;
    observation
        .grade_11_candidates_list
        .push(IndividualCandidate {
            athlete_id,
            name,
            id_result: row
                .get("IDResult")
                .and_then(|value| value.as_u64())
                .ok_or(PageParseError::MissingRowIdResult)?,
            grade_id: 11,
            team_id: row.get("TeamID").and_then(|value| value.as_u64()),
            team_name: row
                .get("TeamName")
                .and_then(|value| value.as_str())
                .map(str::to_owned),
            state: row
                .get("State")
                .and_then(|value| value.as_str())
                .map(str::to_owned),
            country: row
                .get("Country")
                .and_then(|value| value.as_str())
                .map(str::to_owned),
            record_index,
            source_locator: Some(format!("/groupedRankings/{group_index}/{row_index}")),
        });
    Ok((next_index, unresolved, next_count))
}
