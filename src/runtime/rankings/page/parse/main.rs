use super::relay::parse_relay_roster;
use crate::domain::identity::AthleteId;
use crate::domain::name::CanonicalName;
use crate::runtime::rankings::types::{
    ExpectedPageContext, IndividualCandidate, PageObservation, RankingRowObservation,
};

const MAX_PAGE_ROWS: usize = 1_024;
pub(super) const MAX_PAGE_CANDIDATES: usize = 1_024;

/// Parse a source rankings page response with full scope validation.
pub fn parse_page_response(
    raw: &serde_json::Value,
    expected: &ExpectedPageContext<'_>,
) -> Result<PageObservation, PageParseError> {
    let mut observation = PageObservation::default();
    let clamped = validate_scope(raw, expected, &mut observation)?;
    let raw_min_count = raw
        .get("minCount")
        .and_then(|value| value.as_u64())
        .ok_or(PageParseError::MissingMinCount)?;
    if clamped {
        // The source answered a request past the listing's last page with the
        // listing's first page: the list is exhausted, and this response owns no
        // rows of the requested page. Publication seals it as the event's
        // terminal page, so page one is not indexed a second time.
        observation.min_count = raw_min_count;
        return Ok(observation);
    }
    let groups = raw
        .get("groupedRankings")
        .and_then(|value| value.as_array())
        .ok_or(PageParseError::MissingGroupedRankings)?;
    let (_, unresolved, candidate_count) = parse_groups(groups, expected, &mut observation)?;
    observation.grade_11_candidates = candidate_count;
    observation.unresolved_individual_identities = unresolved;
    if expected.is_relay {
        observation.total_relay_rows = u64::try_from(observation.id_results.len())
            .map_err(|_| PageParseError::CounterOverflow)?;
        parse_relay_roster(raw, &mut observation)?;
    }
    observation.min_count = raw_min_count;
    Ok(observation)
}

/// Validate the response against the request that produced it. Returns whether
/// the source answered a past-end request with the listing's first page.
fn validate_scope(
    raw: &serde_json::Value,
    expected: &ExpectedPageContext<'_>,
    observation: &mut PageObservation,
) -> Result<bool, PageParseError> {
    let div = raw.get("division").ok_or(PageParseError::MissingDivision)?;
    let div_id = div
        .get("ID")
        .and_then(|value| value.as_u64())
        .ok_or(PageParseError::MissingDivisionId)?;
    if div_id != expected.division_id {
        return Err(PageParseError::DivisionMismatch {
            expected: expected.division_id,
            actual: div_id,
        });
    }
    observation.division_id = Some(div_id);
    let season_id = div
        .get("SeasonID")
        .and_then(|value| value.as_u64())
        .ok_or(PageParseError::MissingSeasonId)?;
    if season_id != expected.season_id {
        return Err(PageParseError::SeasonMismatch {
            expected: expected.season_id,
            actual: season_id,
        });
    }
    observation.season_id = Some(season_id);
    validate_division_metadata(div)?;
    validate_request_metadata(raw, expected, observation)
}

fn validate_division_metadata(div: &serde_json::Value) -> Result<(), PageParseError> {
    let base_div = div.get("BaseDiv").ok_or(PageParseError::MissingBaseDiv)?;
    let country = base_div
        .get("Country")
        .and_then(|value| value.as_str())
        .ok_or(PageParseError::MissingCountry)?;
    if country != "USA" {
        return Err(PageParseError::WrongCountry {
            expected: "USA".to_owned(),
            actual: country.to_owned(),
        });
    }
    let level = base_div
        .get("Level")
        .and_then(|value| value.as_u64())
        .ok_or(PageParseError::MissingLevel)?;
    if level != 4 {
        return Err(PageParseError::WrongLevel {
            expected: 4,
            actual: level,
        });
    }
    Ok(())
}

fn validate_request_metadata(
    raw: &serde_json::Value,
    expected: &ExpectedPageContext<'_>,
    observation: &mut PageObservation,
) -> Result<bool, PageParseError> {
    let root_gender = raw
        .get("gender")
        .and_then(|value| value.as_str())
        .ok_or(PageParseError::MissingGender)?;
    if root_gender != expected.gender {
        return Err(PageParseError::GenderMismatch {
            expected: expected.gender.to_owned(),
            actual: root_gender.to_owned(),
        });
    }
    observation.request_gender = Some(root_gender.to_owned());
    let root_event_short = raw
        .get("eventShort")
        .and_then(|value| value.as_str())
        .ok_or(PageParseError::MissingEventShort)?;
    if root_event_short != expected.event_short {
        return Err(PageParseError::EventShortMismatch {
            expected: expected.event_short.to_owned(),
            actual: root_event_short.to_owned(),
        });
    }
    observation.event_short = Some(root_event_short.to_owned());
    if let Some(expected_event_id) = expected.event_id {
        let actual_event_id = raw
            .get("eventId")
            .and_then(|value| value.as_u64())
            .ok_or(PageParseError::MissingEventId)?;
        if actual_event_id != expected_event_id {
            return Err(PageParseError::EventIdMismatch {
                expected: expected_event_id,
                actual: actual_event_id,
            });
        }
    }
    validate_settings(raw, expected, observation)
}

fn validate_settings(
    raw: &serde_json::Value,
    expected: &ExpectedPageContext<'_>,
    observation: &mut PageObservation,
) -> Result<bool, PageParseError> {
    let settings = raw.get("settings").ok_or(PageParseError::MissingSettings)?;
    let page = settings
        .get("page")
        .and_then(|value| value.as_u64())
        .ok_or(PageParseError::MissingPage)?;
    let actual_page = u32::try_from(page).map_err(|_| PageParseError::InvalidPageNumber)?;
    // A request past the listing's last page is answered with the listing's
    // first page. Every other page identity conflict stays an error.
    let clamped = actual_page != expected.page;
    if clamped && !(actual_page == 1 && expected.page > 1) {
        return Err(PageParseError::PageMismatch {
            expected: expected.page,
            actual: actual_page,
        });
    }
    if !clamped {
        observation.request_page = Some(actual_page);
    }
    let depth = settings
        .get("depth")
        .and_then(|value| value.as_u64())
        .ok_or(PageParseError::MissingDepth)?;
    if depth == 0 {
        return Err(PageParseError::ZeroPageDepth);
    }
    observation.settings_page_depth = depth;
    let grades = settings
        .get("grades")
        .and_then(|value| value.as_array())
        .ok_or(PageParseError::MissingGrades)?;
    validate_grades(grades, expected)?;
    Ok(clamped)
}

fn validate_grades(
    grades: &[serde_json::Value],
    expected: &ExpectedPageContext<'_>,
) -> Result<(), PageParseError> {
    if expected.is_relay {
        return grades
            .is_empty()
            .then_some(())
            .ok_or(PageParseError::RelayHasGrades);
    }
    let actual = grades
        .iter()
        .map(|value| value.as_u64().ok_or(PageParseError::NonNumericGrade))
        .collect::<Result<Vec<_>, _>>()?;
    let expected_grades = expected
        .requested_grade
        .map(u64::from)
        .into_iter()
        .collect::<Vec<_>>();
    if actual == expected_grades {
        Ok(())
    } else {
        Err(PageParseError::WrongGradeFilter {
            expected: expected_grades,
            actual,
        })
    }
}

fn parse_groups(
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

/// The source publishes `AthleteID: 0` for rows it keeps anonymous; only a
/// positive id is an identity signal.
fn published_athlete_id(row: &serde_json::Value) -> Option<u64> {
    row.get("AthleteID")
        .and_then(|value| value.as_u64())
        .filter(|athlete_id| *athlete_id != 0)
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

fn parse_individual_candidate(
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

#[derive(Debug, thiserror::Error)]
pub enum PageParseError {
    #[error("missing division in response")]
    MissingDivision,
    #[error("missing division ID")]
    MissingDivisionId,
    #[error("missing season ID")]
    MissingSeasonId,
    #[error("missing BaseDiv in division")]
    MissingBaseDiv,
    #[error("missing country in BaseDiv")]
    MissingCountry,
    #[error("missing level in BaseDiv")]
    MissingLevel,
    #[error("missing gender field")]
    MissingGender,
    #[error("missing eventShort field")]
    MissingEventShort,
    #[error("missing eventId")]
    MissingEventId,
    #[error("missing settings")]
    MissingSettings,
    #[error("missing page setting")]
    MissingPage,
    #[error("missing depth setting")]
    MissingDepth,
    #[error("missing grades array")]
    MissingGrades,
    #[error("page number is outside the supported range")]
    InvalidPageNumber,
    #[error("missing numeric minCount")]
    MissingMinCount,
    #[error("missing groupedRankings array")]
    MissingGroupedRankings,
    #[error("relayTeams must be object type")]
    WrongRelayTeamsType,
    #[error("relay team IDResult or RelayTeamID missing")]
    WrongRelayTeamId,
    #[error("Members field must be array type")]
    WrongMembersType,
    #[error("non-array group at index {index}")]
    NonArrayGroup { index: usize },
    #[error("non-numeric grade in settings")]
    NonNumericGrade,
    #[error("missing rowNum in row")]
    MissingRowNum,
    #[error("missing IDResult in row")]
    MissingRowIdResult,
    #[error("row number must be non-zero")]
    ZeroRowNumber,
    #[error("IDResult must be non-zero")]
    ZeroResultId,
    #[error("page row limit exceeded")]
    PageRowsLimitExceeded,
    #[error("page candidate limit exceeded")]
    CandidateLimitExceeded,
    #[error("page counter overflow")]
    CounterOverflow,
    #[error("relay team limit exceeded")]
    RelayTeamLimitExceeded,
    #[error("division ID mismatch: expected {expected}, got {actual}")]
    DivisionMismatch { expected: u64, actual: u64 },
    #[error("season ID mismatch: expected {expected}, got {actual}")]
    SeasonMismatch { expected: u64, actual: u64 },
    #[error("wrong country: expected {expected}, got {actual}")]
    WrongCountry { expected: String, actual: String },
    #[error("wrong level: expected {expected}, got {actual}")]
    WrongLevel { expected: u64, actual: u64 },
    #[error("gender mismatch: expected {expected}, got {actual}")]
    GenderMismatch { expected: String, actual: String },
    #[error("eventShort mismatch: expected {expected}, got {actual}")]
    EventShortMismatch { expected: String, actual: String },
    #[error("eventId mismatch: expected {expected}, got {actual}")]
    EventIdMismatch { expected: u64, actual: u64 },
    #[error("page mismatch: expected {expected}, got {actual}")]
    PageMismatch { expected: u32, actual: u32 },
    #[error("zero page depth in settings")]
    ZeroPageDepth,
    #[error("relay has grades filter: expected empty")]
    RelayHasGrades,
    #[error("wrong grade filter: expected {expected:?}, got {actual:?}")]
    WrongGradeFilter {
        expected: Vec<u64>,
        actual: Vec<u64>,
    },
    #[error("wrong relay join: roster.IDResult={roster_id_result} row.IDResult={row_id_result} roster.RelayTeamID={roster_relay_team_id} row.AthleteID={row_athlete_id}")]
    WrongRelayJoin {
        roster_id_result: u64,
        row_id_result: u64,
        roster_relay_team_id: u64,
        row_athlete_id: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::{parse_page_response, PageParseError};
    use crate::runtime::rankings::ExpectedPageContext;
    use serde_json::json;

    const DIVISION: u64 = 168416;
    const SEASON: u64 = 2026;

    fn expected(page: u32) -> ExpectedPageContext<'static> {
        ExpectedPageContext {
            division_id: DIVISION,
            season_id: SEASON,
            gender: "m",
            event_short: "100m",
            event_id: Some(3),
            is_relay: false,
            requested_grade: Some(11),
            page,
        }
    }

    fn page_body(settings_page: u64) -> serde_json::Value {
        json!({
            "division": {"ID": DIVISION, "SeasonID": SEASON, "BaseDiv": {"Country": "USA", "Level": 4}},
            "gender": "m",
            "eventShort": "100m",
            "eventId": 3,
            "settings": {"page": settings_page, "depth": 100, "grades": [11]},
            "minCount": 24_500,
            "groupedRankings": [[]],
        })
    }

    #[test]
    fn a_request_past_the_listing_end_parses_as_an_empty_terminal_page() {
        // Live shape: the app's own request for page 246 of the outdoor boys
        // grade 11 `100m` list came back carrying `settings.page` 1.
        let observation = parse_page_response(&page_body(1), &expected(246))
            .expect("past-end page parses as the exhausted list");
        assert_eq!(observation.min_count, 24_500);
        assert_eq!(observation.settings_page_depth, 100);
        assert_eq!(observation.request_page, None);
        assert!(observation.grade_11_candidates_list.is_empty());
        assert!(observation.source_rows.is_empty());
    }

    #[test]
    fn a_listing_head_request_keeps_its_page_identity() {
        let observation =
            parse_page_response(&page_body(1), &expected(1)).expect("head page parses");
        assert_eq!(observation.request_page, Some(1));
    }

    #[test]
    fn any_other_page_identity_conflict_stays_an_error() {
        let error = parse_page_response(&page_body(5), &expected(246))
            .expect_err("a mismatched page identity is rejected");
        assert!(matches!(
            error,
            PageParseError::PageMismatch {
                expected: 246,
                actual: 5
            }
        ));
    }

    fn relay_expected(page: u32) -> ExpectedPageContext<'static> {
        ExpectedPageContext {
            division_id: DIVISION,
            season_id: SEASON,
            gender: "m",
            event_short: "4x100m",
            event_id: Some(20),
            is_relay: true,
            requested_grade: None,
            page,
        }
    }

    fn relay_body() -> serde_json::Value {
        json!({
            "division": {"ID": DIVISION, "SeasonID": SEASON, "BaseDiv": {"Country": "USA", "Level": 4}},
            "gender": "m",
            "eventShort": "4x100m",
            "eventId": 20,
            "settings": {"page": 1, "depth": 100, "grades": []},
            "minCount": 1_288,
            "groupedRankings": [[
                // Masked row: the source still serves its roster entry, but the
                // row arrives with `AthleteID: 0`, exactly as the live 4x100m
                // list served it to this session.
                {"rowNum": 1, "IDResult": 11, "AthleteID": 0, "GradeID": 99},
                // Readable row: identity present and the roster joins by both keys.
                {"rowNum": 2, "IDResult": 12, "AthleteID": 21, "GradeID": 99},
            ]],
            "relayTeams": {
                "11": {"IDResult": 11, "RelayTeamID": 999, "Members": [
                    {"SortID": 1, "GradeID": 11, "IDAthlete": 601, "AthleteName": "Masked Member", "Handle": "m"},
                ]},
                "12": {"IDResult": 12, "RelayTeamID": 21, "Members": [
                    {"SortID": 1, "GradeID": 11, "IDAthlete": 501, "AthleteName": "Ada Relay", "Handle": "ada-r"},
                ]},
            },
        })
    }

    #[test]
    fn a_masked_relay_row_is_recorded_unresolved_instead_of_failing_the_page() {
        let observation = parse_page_response(&relay_body(), &relay_expected(1))
            .expect("masked relay rows do not fail the page");
        assert_eq!(observation.rows_missing_roster, 1);
        assert_eq!(observation.rows_with_roster, 1);
        assert_eq!(observation.verified_relay_members.len(), 1);
        assert_eq!(observation.verified_relay_members[0].athlete_id.get(), 501);
        assert_eq!(observation.source_rows[0].roster_present, Some(false));
        assert_eq!(observation.source_rows[1].roster_present, Some(true));
    }
}
