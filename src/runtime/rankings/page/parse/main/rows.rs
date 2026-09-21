use super::candidate::{parse_individual_candidate, published_athlete_id};
use super::error::PageParseError;
use super::MAX_PAGE_ROWS;
use crate::runtime::rankings::types::{
    ExpectedPageContext, PageObservation, RankingRowObservation,
};

pub(super) fn parse_groups(
    groups: &[serde_json::Value],
    expected: &ExpectedPageContext<'_>,
    observation: &mut PageObservation,
) -> Result<(u64, u64, u64), PageParseError> {
    groups
        .iter()
        .enumerate()
        .try_fold((0_u64, 0_u64, 0_u64), |state, (group_index, group)| {
            let rows = group
                .as_array()
                .ok_or(PageParseError::NonArrayGroup { index: group_index })?;
            rows.iter()
                .enumerate()
                .try_fold(state, |state, (row_index, row)| {
                    parse_row(row, group_index, row_index, expected, observation, state)
                })
        })
}

fn parse_row(
    row: &serde_json::Value,
    group_index: usize,
    row_index: usize,
    expected: &ExpectedPageContext<'_>,
    observation: &mut PageObservation,
    (candidate_index, unresolved, candidate_count): (u64, u64, u64),
) -> Result<(u64, u64, u64), PageParseError> {
    if observation.source_rows.len() >= MAX_PAGE_ROWS {
        return Err(PageParseError::PageRowsLimitExceeded);
    }
    let row_number = row
        .get("rowNum")
        .and_then(|value| value.as_u64())
        .ok_or(PageParseError::MissingRowNum)?;
    if row_number == 0 {
        return Err(PageParseError::ZeroRowNumber);
    }
    let id_result = row
        .get("IDResult")
        .and_then(|value| value.as_u64())
        .ok_or(PageParseError::MissingRowIdResult)?;
    if id_result == 0 {
        return Err(PageParseError::ZeroResultId);
    }
    observation.row_count = observation
        .row_count
        .checked_add(1)
        .ok_or(PageParseError::CounterOverflow)?;
    observation.source_rows.push(RankingRowObservation {
        result_id: id_result,
        row_number,
        roster_present: None,
    });
    if expected.is_relay {
        observation.id_results.push(id_result);
    } else if published_athlete_id(row).is_some() {
        // The observation is content-addressed, so rows the source publishes
        // with a real athlete id keep contributing here; an anonymous row
        // (`AthleteID: 0`) stays counted in `source_rows` without joining an
        // identity and without failing the page.
        observation.id_results.push(id_result);
    }
    if expected.is_relay || row.get("GradeID").and_then(|value| value.as_u64()) != Some(11) {
        return Ok((candidate_index, unresolved, candidate_count));
    }
    parse_individual_candidate(
        row,
        group_index,
        row_index,
        observation,
        (candidate_index, unresolved, candidate_count),
    )
}
