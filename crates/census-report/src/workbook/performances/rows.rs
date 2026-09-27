
use super::super::cells::{row, Cell};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct PerformanceRow {
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
    pub(super) fn cells(&self) -> Vec<Cell> {
        row!(
            Cell::text(self.id.clone()),
            Cell::text(self.athlete_id.clone()),
            Cell::text(self.athlete.clone()),
            Cell::text(self.school.clone()),
            self.grad_year
                .map(|year| Cell::Number(f64::from(year)))
                .unwrap_or(Cell::Empty),
            Cell::text(self.meet_id.clone()),
            Cell::text(self.meet.clone()),
            Cell::text(self.date.clone()),
            Cell::text(self.state.clone().unwrap_or_default()),
            Cell::text(self.sport.clone()),
            Cell::text(self.event.clone()),
            Cell::text(self.mark.clone()),
            self.normalized.map(Cell::Number).unwrap_or(Cell::Empty),
            Cell::text(self.timing.clone().unwrap_or_default()),
            self.wind_mps.map(Cell::Number).unwrap_or(Cell::Empty),
            Cell::text(self.round.clone().unwrap_or_default()),
            self.place
                .map(|place| Cell::Number(f64::from(place)))
                .unwrap_or(Cell::Empty),
            Cell::text(self.source.clone()),
            Cell::text(self.source_result.clone()),
            Cell::text(self.source_url.clone()),
        )
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
