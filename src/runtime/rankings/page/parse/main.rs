use crate::domain::identity::AthleteId;
use crate::domain::name::CanonicalName;
use crate::runtime::rankings::types::{
    ExpectedPageContext, IndividualCandidate, PageObservation, RankingRowObservation,
};
use super::relay::parse_relay_roster;

/// Parse a source rankings page response with full scope validation.
pub fn parse_page_response(
    raw: &serde_json::Value,
    expected: &ExpectedPageContext<'_>,
) -> Result<PageObservation, PageParseError> {
    let mut observation = PageObservation::default();

    // Validate division
    let div = raw.get("division").ok_or(PageParseError::MissingDivision)?;
    let div_id = div.get("ID").and_then(|v| v.as_u64()).ok_or(PageParseError::MissingDivisionId)?;
    if div_id != expected.division_id {
        return Err(PageParseError::DivisionMismatch {
            expected: expected.division_id,
            actual: div_id,
        });
    }
    observation.division_id = Some(div_id);

    // Validate season
    let season_id = div.get("SeasonID").and_then(|v| v.as_u64()).ok_or(PageParseError::MissingSeasonId)?;
    if season_id != expected.season_id {
        return Err(PageParseError::SeasonMismatch {
            expected: expected.season_id,
            actual: season_id,
        });
    }
    observation.season_id = Some(season_id);

    // BaseDiv REQUIRED: validate country and level
    let base_div = div.get("BaseDiv").ok_or(PageParseError::MissingBaseDiv)?;
    let country = base_div.get("Country").and_then(|v| v.as_str()).ok_or(PageParseError::MissingCountry)?;
    if country != "USA" {
        return Err(PageParseError::WrongCountry {
            expected: "USA".to_string(),
            actual: country.to_string(),
        });
    }
    let level = base_div.get("Level").and_then(|v| v.as_u64()).ok_or(PageParseError::MissingLevel)?;
    if level != 4 {
        return Err(PageParseError::WrongLevel { expected: 4, actual: level });
    }

    // Validate root gender
    let root_gender = raw.get("gender").and_then(|v| v.as_str()).ok_or(PageParseError::MissingGender)?;
    if root_gender != expected.gender {
        return Err(PageParseError::GenderMismatch {
            expected: expected.gender.to_string(),
            actual: root_gender.to_string(),
        });
    }
    observation.request_gender = Some(root_gender.to_owned());

    // Validate eventShort
    let root_event_short = raw.get("eventShort").and_then(|v| v.as_str()).ok_or(PageParseError::MissingEventShort)?;
    if root_event_short != expected.event_short {
        return Err(PageParseError::EventShortMismatch {
            expected: expected.event_short.to_string(),
            actual: root_event_short.to_string(),
        });
    }
    observation.event_short = Some(root_event_short.to_owned());

    // Validate eventId when expected
    if let Some(expected_eid) = expected.event_id {
        let actual_eid = raw.get("eventId").and_then(|v| v.as_u64()).ok_or(PageParseError::MissingEventId)?;
        if actual_eid != expected_eid {
            return Err(PageParseError::EventIdMismatch {
                expected: expected_eid,
                actual: actual_eid,
            });
        }
    }

    // Validate settings
    let settings = raw.get("settings").ok_or(PageParseError::MissingSettings)?;

    // page REQUIRED: use u64 (no lossy i64)
    let page = settings.get("page").and_then(|v| v.as_u64()).ok_or(PageParseError::MissingPage)?;
    let actual_page = u32::try_from(page).map_err(|_| PageParseError::InvalidPageNumber)?;
    if actual_page != expected.page {
        return Err(PageParseError::PageMismatch { expected: expected.page, actual: actual_page });
    }
    observation.request_page = Some(actual_page);
    let depth = settings.get("depth").and_then(|v| v.as_u64()).ok_or(PageParseError::MissingDepth)?;
    if depth == 0 {
        return Err(PageParseError::ZeroPageDepth);
    }
    observation.settings_page_depth = depth;

    // grades REQUIRED: must match expected
    let grades = settings.get("grades").and_then(|v| v.as_array()).ok_or(PageParseError::MissingGrades)?;
    if expected.is_relay {
        // Relay: grades must be empty array
        if !grades.is_empty() {
            return Err(PageParseError::RelayHasGrades);
        }
    } else if let Some(req_grade) = expected.requested_grade {
        // Individual with explicit grade: grades must exactly match
        let g: Vec<u64> = grades
            .iter()
            .map(|v| v.as_u64().ok_or(PageParseError::NonNumericGrade))
            .collect::<Result<Vec<_>, _>>()?;
        if g != vec![req_grade as u64] {
            return Err(PageParseError::WrongGradeFilter {
                expected: vec![req_grade as u64],
                actual: g,
            });
        }
    } else {
        // Historical all-grade (requested_grade==None): grades must be empty
        if !grades.is_empty() {
            return Err(PageParseError::WrongGradeFilter {
                expected: vec![],
                actual: grades.iter().filter_map(|v| v.as_u64()).collect(),
            });
        }
    }

    // Parse groupedRankings array
    let groups = raw
        .get("groupedRankings")
        .and_then(|v| v.as_array())
        .ok_or(PageParseError::MissingGroupedRankings)?;

    // Read raw minCount from response
    let raw_min_count = raw
        .get("minCount")
        .and_then(|v| v.as_u64())
        .ok_or(PageParseError::MissingMinCount)?;

    let mut candidate_idx = 0u64;
    let mut unresolved_individual = 0u64;
    let mut grade_11_candidates_count = 0u64;

    for (gi, group) in groups.iter().enumerate() {
        let rows = group.as_array().ok_or(PageParseError::NonArrayGroup { index: gi })?;
        for (ri, rv) in rows.iter().enumerate() {
            // Source pointer: JSON pointer format with leading /
            let source_ptr = format!("/groupedRankings/{}/{}", gi, ri);

            // row_number from raw rowNum field (not fabricated counter)
            let raw_row_num = rv.get("rowNum").and_then(|v| v.as_u64()).ok_or(PageParseError::MissingRowNum)?;
            observation.row_count += 1;

            // IDResult REQUIRED on each row
            let id_result = rv.get("IDResult").and_then(|v| v.as_u64()).ok_or(PageParseError::MissingRowIdResult)?;

            // Source row observation
            observation.source_rows.push(RankingRowObservation {
                result_id: id_result,
                row_number: raw_row_num,
                roster_present: None, // filled by relay parser
            });

            // Track id_results and athlete_ids for relay joins
            let athlete_id_val = rv.get("AthleteID").and_then(|v| v.as_u64());
            if let Some(aid) = athlete_id_val {
                observation.id_results.push(id_result);
                observation.row_athlete_ids.insert(id_result.to_string(), aid);
            }

            // Individual candidate extraction (non-relay only, GradeID=11)
            if !expected.is_relay {
                if let Some(grade_id) = rv.get("GradeID").and_then(|v| v.as_u64()) {
                    if grade_id == 11 {
                        if let (Some(aid), Some(name_raw)) = (
                            rv.get("AthleteID").and_then(|v| v.as_u64()),
                            rv.get("AthleteName").and_then(|v| v.as_str()),
                        ) {
                            // Must have IDResult for individual candidate
                            // Tuple match: both results must be Ok
                            let (name_res, athlete_res) = (
                                CanonicalName::parse(name_raw),
                                AthleteId::new(aid),
                            );
                            match (name_res, athlete_res) {
                                (Ok(name), Ok(athlete_id)) => {
                                    grade_11_candidates_count += 1;
                                    observation.grade_11_candidates_list.push(IndividualCandidate {
                                        athlete_id,
                                        name,
                                        id_result,
                                        grade_id,
                                        team_id: rv.get("TeamID").and_then(|v| v.as_u64()),
                                        team_name: rv.get("TeamName")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_owned()),
                                        state: rv.get("State").and_then(|v| v.as_str()).map(|s| s.to_owned()),
                                        country: rv.get("Country").and_then(|v| v.as_str()).map(|s| s.to_owned()),
                                        source_locator: Some(source_ptr),
                                        source_row: Some(rv.clone()),
                                        record_index: candidate_idx,
                                    });
                                    candidate_idx += 1;
                                }
                                _ => {
                                    // Missing or invalid name/ID increments unresolved
                                    unresolved_individual += 1;
                                }
                            }
                        } else {
                            // Missing AthleteID or AthleteName for GradeID=11 row
                            unresolved_individual += 1;
                        }
                    }
                }
            }
        }
    }

    observation.grade_11_candidates = grade_11_candidates_count;
    observation.unresolved_individual_identities = unresolved_individual;

    // Relay roster parsing
    if expected.is_relay {
        observation.total_relay_rows = observation.id_results.len() as u64;
        parse_relay_roster(raw, &mut observation)?;
    }

    // min_count from raw JSON minCount field
    observation.min_count = raw_min_count;

    Ok(observation)
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
    WrongGradeFilter { expected: Vec<u64>, actual: Vec<u64> },
    #[error("wrong relay join: roster.IDResult={roster_id_result} row.IDResult={row_id_result} roster.RelayTeamID={roster_relay_team_id} row.AthleteID={row_athlete_id}")]
    WrongRelayJoin {
        roster_id_result: u64,
        row_id_result: u64,
        roster_relay_team_id: u64,
        row_athlete_id: u64,
    },
}
