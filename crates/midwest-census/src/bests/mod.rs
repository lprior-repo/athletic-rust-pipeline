//! Per-athlete best marks - the platform's own "PR" reduction.
//!
//! A best mark is a deterministic reduction over rows the platform already reconciled, so nothing here
//! fetches anything: for every `(athlete, event)` pair it keeps the mark that wins on that event's own
//! scale - the lowest time, the highest distance, height or score - and carries along the meet, date,
//! place, wind reading and timing method that produced it. A recruiting row can therefore cite the
//! performance instead of asserting a number.
//!
//! Two deliberate choices:
//!
//! * **Relays are not personal bests.** A 4x400 split is a squad mark; including it would put a
//!   number in an athlete's PR column that the athlete did not run alone.
//! * **Marks are compared only within their own measure.** A time is never compared against a
//!   distance, so an unparsed [`Mark::Raw`](census_domain::model::Mark::Raw) value is carried but never
//!   chosen as a best.

use crate::report::Scope;
use census_domain::MeetState;
use serde::Serialize;

mod events;
mod measure;
mod notation;
mod reduce;
mod write;

#[cfg(test)]
use crate::store::Store;
#[cfg(test)]
use census_domain::model::{EventKind, Mark};

pub use events::{is_relay, sport_of};
pub use measure::Measure;
pub use notation::{format_time, mark_text};
pub use reduce::build;
pub use write::write;

/// One athlete's best mark in one event, with the performance that produced it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BestResult {
    pub athlete_id: String,
    pub name: String,
    pub school: String,
    /// The meet's jurisdiction as it prints: the USPS code, or the store's `??` sentinel when the
    /// meet never states where it was held. A label, never a fabricated state.
    pub state: MeetState,
    pub grad_year: i16,
    pub gender: String,
    pub sport: String,
    pub event: String,
    pub best_mark: String,
    pub best_value: f64,
    pub measure: String,
    pub date: String,
    pub meet: String,
    pub place: Option<u16>,
    pub wind_mps: Option<f64>,
    pub timing: Option<String>,
    /// How many marks this athlete has in this event, i.e. how much the best rests on.
    pub marks_in_event: usize,
    pub profile_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub scope: Scope,
    /// Cohort selector; `None` reduces every athlete in scope.
    pub grad_year: Option<i16>,
    pub limit: Option<usize>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        }
    }
}

#[cfg(test)]
mod tests;
