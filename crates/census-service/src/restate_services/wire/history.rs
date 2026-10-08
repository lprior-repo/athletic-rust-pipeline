use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::census::MeetCensus;
use crate::restate_services::results_arms::ResultsStageOutcome;

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("invalid census as-of date: {0}")]
    Date(#[from] chrono::ParseError),
    #[error("as-of calendar year {0} is outside the supported range")]
    AsOfYear(i32),
    #[error("history must be a finite cohort window ending by its immutable as-of date")]
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "HistoryWire")]
pub struct HistoryWindow {
    first_calendar_year: u16,
    last_calendar_year: u16,
    as_of: chrono::NaiveDate,
}

#[derive(Deserialize)]
struct HistoryWire {
    first_calendar_year: u16,
    last_calendar_year: u16,
    as_of: String,
}

impl HistoryWindow {
    pub fn cohort(as_of: &str) -> Result<Self, HistoryError> {
        let date = chrono::NaiveDate::parse_from_str(as_of, "%Y-%m-%d")?;
        let last = u16::try_from(date.year()).map_err(|_| HistoryError::AsOfYear(date.year()))?;
        Self::from_date(2023, last.min(2027), date)
    }

    pub fn new(first: u16, last: u16, as_of: &str) -> Result<Self, HistoryError> {
        Self::from_date(
            first,
            last,
            chrono::NaiveDate::parse_from_str(as_of, "%Y-%m-%d")?,
        )
    }

    fn from_date(first: u16, last: u16, date: chrono::NaiveDate) -> Result<Self, HistoryError> {
        if first < 2020
            || first > last
            || last > 2027
            || last.checked_sub(first).is_none_or(|span| span > 7)
            || i32::from(last) > date.year()
        {
            return Err(HistoryError::Window);
        }
        Ok(Self {
            first_calendar_year: first,
            last_calendar_year: last,
            as_of: date,
        })
    }

    pub fn years(&self) -> std::ops::RangeInclusive<u16> {
        self.first_calendar_year..=self.last_calendar_year
    }

    pub fn as_of(&self) -> chrono::NaiveDate {
        self.as_of
    }

    pub fn admits_date(&self, raw: &str) -> bool {
        chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d").is_ok_and(|date| {
            date.year() >= i32::from(self.first_calendar_year)
                && date.year() <= i32::from(self.last_calendar_year)
                && date <= self.as_of
        })
    }
}

impl TryFrom<HistoryWire> for HistoryWindow {
    type Error = HistoryError;
    fn try_from(value: HistoryWire) -> Result<Self, Self::Error> {
        Self::new(
            value.first_calendar_year,
            value.last_calendar_year,
            &value.as_of,
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoricalProgress {
    pub meets: BTreeMap<u16, MeetCensus>,
    pub results: BTreeMap<u16, ResultsStageOutcome>,
}

impl HistoricalProgress {
    pub fn meets_terminal(&self, window: &HistoryWindow) -> bool {
        window
            .years()
            .all(|year| self.meets.get(&year).is_some_and(MeetCensus::is_terminal))
    }
    pub fn results_terminal(&self, window: &HistoryWindow) -> bool {
        window.years().all(|year| {
            self.results
                .get(&year)
                .is_some_and(ResultsStageOutcome::is_terminal)
        })
    }
}
