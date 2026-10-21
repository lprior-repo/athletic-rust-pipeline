use super::*;
use census_domain::result_calendar_correspondence::RoncalliSeasonEventSelectionV1;
pub(super) const REVIEW: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/participation_evidence/roncalli/tests/fixtures/season-review.html"
));
pub(super) const SCHEDULE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/participation_evidence/roncalli/tests/fixtures/schedule.html"
));
const HEADERS: &[u8] = b"HTTP/2 200\r\ncontent-type: text/html; charset=UTF-8\r\nX-Controlled-Fixture: retained-genuine-Roncalli-bodies\r\n\r\n";
pub(super) fn packet() -> TestResult<IndependentParticipationPacketV1> {
    Ok(
        IndependentParticipationPacketV1::from_roncalli_season_event(
            captures::capture(
                "https://athletics.roncalli.org/girls-track-field/",
                REVIEW.as_bytes(),
                HEADERS,
                "2026-10-21T12:00:00Z",
                "2026-10-21T12:00:01Z",
            )?,
            captures::capture(
                "https://athletics.roncalli.org/girls-track-field/schedules/",
                SCHEDULE.as_bytes(),
                HEADERS,
                "2026-10-21T12:00:00Z",
                "2026-10-21T12:00:01Z",
            )?,
            RoncalliSeasonEventSelectionV1 {
                status_subject_start: 76865,
                status_subject_end: 76882,
                calendar_recap_id: "event-3387-recap".into(),
            },
        )?,
    )
}
