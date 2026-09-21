mod candidate;
mod error;
mod rows;
mod scope;
use super::relay::parse_relay_roster;
use crate::runtime::rankings::types::{ExpectedPageContext, PageObservation};
use rows::parse_groups;
use scope::validate_scope;

pub use error::PageParseError;

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
