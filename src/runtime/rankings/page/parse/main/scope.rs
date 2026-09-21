use super::error::PageParseError;
use crate::runtime::rankings::types::{ExpectedPageContext, PageObservation};

/// Validate the response against the request that produced it. Returns whether
/// the source answered a past-end request with the listing's first page.
pub(super) fn validate_scope(
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
