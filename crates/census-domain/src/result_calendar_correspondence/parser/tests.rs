use super::*;
use crate::model::ProfileDate;
use crate::result_calendar_correspondence::{
    RoncalliSeasonEventProofContextV1, RoncalliSeasonEventSelectionV1,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn parser_contract_is_closed_and_preserves_exact_legacy_bytes() -> TestResult {
    for parser in [
        ResultCalendarSourceParserV1::MileSplitDcc,
        ResultCalendarSourceParserV1::TfrrsRoncalli,
    ] {
        check!(eq; ResultCalendarSourceParserV1::from_contract(parser.contract()), Some(parser));
        let mut changed = parser.contract().to_vec();
        changed.push(b'\n');
        check!(ResultCalendarSourceParserV1::from_contract(&changed).is_none());
    }
    check!(eq; ResultCalendarSourceParserV1::MileSplitDcc.contract(), RESULT_CALENDAR_PARSER_CONTRACT_V1);
    check!(ResultCalendarSourceParserV1::TfrrsRoncalli
        .supports_scope(Sport::IndoorTrack, Gender::Girls));
    check!(!ResultCalendarSourceParserV1::TfrrsRoncalli
        .supports_scope(Sport::OutdoorTrack, Gender::Girls));
    check!(eq; ResultCalendarSourceParserV1::TfrrsRoncalli.documentary_interpretation("roncalli_girls_track_season_review_v1", false, true), Some("roncalli_girls_track_season_review_calendar_day_v1"));
    check!(ResultCalendarSourceParserV1::TfrrsRoncalli
        .documentary_interpretation("mhsaa_article_v1", false, true)
        .is_none());
    check!(ResultCalendarSourceParserV1::MileSplitDcc
        .documentary_interpretation("roncalli_girls_track_season_review_v1", false, true)
        .is_none());
    Ok(())
}

#[test]
fn selection_is_bounded_and_pair_preimage_binds_both_originals_and_day() -> TestResult {
    let selection = RoncalliSeasonEventSelectionV1 {
        status_subject_start: 76865,
        status_subject_end: 76882,
        calendar_recap_id: "event-3387-recap".into(),
    };
    check!(selection.is_bounded());
    for (start, end, recap) in [
        (1, 1, "event-1-recap"),
        (5, 2, "event-1-recap"),
        (0, 513, "event-1-recap"),
        (0, 2_097_153, "event-1-recap"),
        (1, 2, "event-01-recap"),
        (1, 2, "event-1-recap\n"),
        (1, 2, "event-12345678901234567-recap"),
    ] {
        check!(!RoncalliSeasonEventSelectionV1 {
            status_subject_start: start,
            status_subject_end: end,
            calendar_recap_id: recap.into()
        }
        .is_bounded());
    }
    let day = ProfileDate::parse("2026-03-11")?;
    let value = selection.interpretation_preimage(RoncalliSeasonEventProofContextV1 {
        status_body_sha: "status-sha",
        schedule_body_sha: "calendar-sha",
        status_locator: "html_bytes:76865:76882",
        calendar_locator: "html_bytes:79425:80652",
        day: &day,
    });
    check!(eq; value, "roncalli_girls_track_season_review_calendar_day_v1\nstatus-sha\ncalendar-sha\nhtml_bytes:76865:76882\nhtml_bytes:79425:80652\n2026-03-11\nevent-3387-recap");
    Ok(())
}
