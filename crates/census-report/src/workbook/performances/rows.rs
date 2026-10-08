use super::super::cells::Cell;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceRow {
    pub(super) id: String,
    pub(super) athlete_id: String,
    pub(super) athlete: String,
    pub(super) school: String,
    pub(super) grad_year: Option<i16>,
    pub(super) meet_id: String,
    pub(super) meet: String,
    pub(super) date: String,
    pub(super) state: Option<String>,
    pub(super) sport: String,
    pub(super) event: String,
    pub(super) mark: String,
    pub(super) normalized: Option<f64>,
    pub(super) timing: Option<String>,
    pub(super) wind_mps: Option<f64>,
    pub(super) round: Option<String>,
    pub(super) place: Option<u16>,
    pub(super) source: String,
    pub(super) source_result: String,
    pub(super) source_url: String,
}

impl PerformanceRow {
    pub fn values(&self) -> [ProjectedValue<'_>; 20] {
        use ProjectedValue::{Number, Text};
        [
            Text(&self.id),
            Text(&self.athlete_id),
            Text(&self.athlete),
            Text(&self.school),
            Number(self.grad_year.map(f64::from)),
            Text(&self.meet_id),
            Text(&self.meet),
            Text(&self.date),
            Text(self.state.as_deref().map_or("", |state| state)),
            Text(&self.sport),
            Text(&self.event),
            Text(&self.mark),
            Number(self.normalized),
            Text(self.timing.as_deref().map_or("", |timing| timing)),
            Number(self.wind_mps),
            Text(self.round.as_deref().map_or("", |round| round)),
            Number(self.place.map(f64::from)),
            Text(&self.source),
            Text(&self.source_result),
            Text(&self.source_url),
        ]
    }

    pub(super) fn cells(&self) -> Vec<Cell> {
        self.values()
            .into_iter()
            .map(|value| match value {
                ProjectedValue::Text(text) => Cell::text(text),
                ProjectedValue::Number(Some(number)) => Cell::Number(number),
                ProjectedValue::Number(None) => Cell::Empty,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ProjectedValue<'a> {
    Text(&'a str),
    Number(Option<f64>),
}

impl ProjectedValue<'_> {
    pub fn matches(self, actual: &str) -> bool {
        match self {
            Self::Text(expected) => actual == expected,
            Self::Number(None) => actual.is_empty(),
            Self::Number(Some(expected)) => actual
                .parse::<f64>()
                .is_ok_and(|value| value.is_finite() && value == expected),
        }
    }
}

pub(super) fn sheet_order(left: &PerformanceRow, right: &PerformanceRow) -> Ordering {
    left.school
        .cmp(&right.school)
        .then_with(|| left.date.cmp(&right.date))
        .then_with(|| left.athlete.cmp(&right.athlete))
        .then_with(|| left.athlete_id.cmp(&right.athlete_id))
        .then_with(|| left.event.cmp(&right.event))
        .then_with(|| left.id.cmp(&right.id))
}
