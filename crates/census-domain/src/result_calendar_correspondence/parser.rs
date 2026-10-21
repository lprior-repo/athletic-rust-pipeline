use super::RESULT_CALENDAR_PARSER_CONTRACT_V1;
use crate::model::{Gender, Sport};

pub const RESULT_CALENDAR_TFRRS_RONCALLI_PARSER_CONTRACT_V1: &[u8] = br#"{"version":1,"contract":"reviewed-tfrrs-roncalli-season-calendar-day-v1","row_interpretation":"retained-tfrrs-list-row/v1","documentary_interpretation":"roncalli_girls_track_season_review_calendar_day_v1","authority":["scholastic-context","exact-original-day-in-season"],"full-window":"unasserted","cohort":"unasserted","school-identity":"unasserted","ncaa":"unasserted","raw-labels":"unchanged"}"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultCalendarSourceParserV1 {
    MileSplitDcc,
    TfrrsRoncalli,
}

#[cfg(test)]
mod tests;

impl ResultCalendarSourceParserV1 {
    pub fn from_contract(bytes: &[u8]) -> Option<Self> {
        if bytes == RESULT_CALENDAR_PARSER_CONTRACT_V1 {
            Some(Self::MileSplitDcc)
        } else if bytes == RESULT_CALENDAR_TFRRS_RONCALLI_PARSER_CONTRACT_V1 {
            Some(Self::TfrrsRoncalli)
        } else {
            None
        }
    }

    pub fn contract(self) -> &'static [u8] {
        match self {
            Self::MileSplitDcc => RESULT_CALENDAR_PARSER_CONTRACT_V1,
            Self::TfrrsRoncalli => RESULT_CALENDAR_TFRRS_RONCALLI_PARSER_CONTRACT_V1,
        }
    }

    pub fn row_interpretation(self) -> &'static str {
        match self {
            Self::MileSplitDcc => "retained-milesplit-html-result-row/v1",
            Self::TfrrsRoncalli => "retained-tfrrs-list-row/v1",
        }
    }

    pub fn result_namespace(self) -> &'static str {
        match self {
            Self::MileSplitDcc => "milesplit_result_document_v1",
            Self::TfrrsRoncalli => "tfrrs_result_document_v1",
        }
    }

    pub fn supports_scope(self, sport: Sport, category: Gender) -> bool {
        match self {
            Self::MileSplitDcc => sport == Sport::OutdoorTrack && category == Gender::Boys,
            Self::TfrrsRoncalli => sport == Sport::IndoorTrack && category == Gender::Girls,
        }
    }

    pub fn documentary_interpretation(
        self,
        namespace: &str,
        result_row: bool,
        roncalli_season_event: bool,
    ) -> Option<&'static str> {
        match (self, namespace, result_row, roncalli_season_event) {
            (Self::MileSplitDcc, "mhsaa_article_v1", false, false) => {
                Some("mhsaa_subject_dcc_event_day_v1")
            }
            (Self::MileSplitDcc, "milesplit_result_document_v1", true, false) => {
                Some("milesplit_original_student_grade_dcc_event_day_v1")
            }
            (Self::TfrrsRoncalli, "roncalli_girls_track_season_review_v1", false, true) => {
                Some("roncalli_girls_track_season_review_calendar_day_v1")
            }
            _ => None,
        }
    }
}
