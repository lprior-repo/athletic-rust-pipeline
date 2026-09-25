//! The §52 row: one row per stored performance, and the order the sheets publish.
//!
//! [`PerformanceRow`] is assembled by [`super::join`], which copies the athlete (name, graduation
//! year), the athlete's school, the meet (name, venue state) and the event (label, round) out of the
//! canonical rows the store holds, all under the requested scope. [`PerformanceRow::cells`] is the
//! one place a cell is formatted: `Mark` is the source's own notation (`10.94`, `4:41.23`, `5' 4"`),
//! `Normalized Mark` is the same mark on its own comparable scale (seconds, metres or points) and
//! stays blank for a mark the census has not parsed yet, `Sport` is the event family the `Best
//! results` sheet already publishes, `Timing`, `Wind`, `Round` and `Place` are the performance's own
//! published conditions, and `Source`, `Source ResultID` and `Source URL` are the observation the row
//! rests on: the lexicographically first source that observed it, the provider-local result key, and
//! that source's URL.
//!
//! Relay legs are included. They are deliberately not personal bests (see [`crate::bests`]), but §52
//! asks for every performance, and a 4x400 leg is one.

use super::super::cells::{row, Cell};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// One §52 row, joined and formatted; the sheet writer only copies these fields out.
///
/// The fields are `pub(super)` so the module's tests can assert the join column by column; nothing
/// outside this module tree reads them.
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
    /// The row's cells, in [`COLUMNS`](super::COLUMNS) order.
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

/// The order the sheets publish and therefore the order they are cut into partitions: school, date,
/// athlete, event, then the unique performance id, which breaks every remaining tie.
///
/// School name comes first so a contiguous slice of the school-name universe is a contiguous slice of
/// this order, which is what lets [`super::spill`] sort one range at a time.
pub(super) fn sheet_order(left: &PerformanceRow, right: &PerformanceRow) -> Ordering {
    left.school
        .cmp(&right.school)
        .then_with(|| left.date.cmp(&right.date))
        .then_with(|| left.athlete.cmp(&right.athlete))
        .then_with(|| left.athlete_id.cmp(&right.athlete_id))
        .then_with(|| left.event.cmp(&right.event))
        .then_with(|| left.id.cmp(&right.id))
}
