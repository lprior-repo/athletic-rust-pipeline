use crate::model::ProfileDate;
use serde::{Deserialize, Serialize};

pub const RONCALLI_SEASON_EVENT_INTERPRETATION_V1: &str =
    "roncalli_girls_track_season_review_calendar_day_v1";

pub struct RoncalliSeasonEventProofContextV1<'a> {
    pub status_body_sha: &'a str,
    pub schedule_body_sha: &'a str,
    pub status_locator: &'a str,
    pub calendar_locator: &'a str,
    pub day: &'a ProfileDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoncalliSeasonEventSelectionV1 {
    pub status_subject_start: usize,
    pub status_subject_end: usize,
    pub calendar_recap_id: String,
}

impl RoncalliSeasonEventSelectionV1 {
    pub fn interpretation_preimage(
        &self,
        context: RoncalliSeasonEventProofContextV1<'_>,
    ) -> String {
        format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}",
            RONCALLI_SEASON_EVENT_INTERPRETATION_V1,
            context.status_body_sha,
            context.schedule_body_sha,
            context.status_locator,
            context.calendar_locator,
            context.day.as_str(),
            self.calendar_recap_id
        )
    }

    pub fn is_bounded(&self) -> bool {
        let Some(length) = self
            .status_subject_end
            .checked_sub(self.status_subject_start)
        else {
            return false;
        };
        let Some(digits) = self
            .calendar_recap_id
            .strip_prefix("event-")
            .and_then(|value| value.strip_suffix("-recap"))
        else {
            return false;
        };
        length > 0
            && length <= 512
            && self.status_subject_end <= 2 * 1024 * 1024
            && !digits.is_empty()
            && digits.len() <= 16
            && !digits.starts_with('0')
            && digits.bytes().all(|value| value.is_ascii_digit())
    }
}
