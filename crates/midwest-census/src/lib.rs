//! `midwest-census` — independent Midwest high-school TF/XC recruiting census.
//!
//! Goal: build the recruiting graph **without** broad Athletic.net crawling. Athletic.net stays the
//! deepest performance source, but it is entered only through ids obtained from cheaper sources
//! (MileSplit rosters, state associations, timing providers), and only for athletes that another
//! source has already identified.
//!
//! Layers:
//!
//! * [`model`] — canonical entities with locally minted deterministic ids, grad-year cohorts and
//!   `ObservedGrade` evidence instead of mutable grade fields.
//! * [`net`] — robots-enforcing, cache-first, per-host-rate-limited fetcher with request evidence.
//! * [`store`] — append-only entity logs plus streaming consolidation into snapshots.
//! * [`sources`] — one module per provider (MileSplit first, then associations/timers).
//! * [`census`] — resumable orchestration.
//! * [`report`] — measured census output (`report.json`, per-state CSV).

pub mod census;
pub mod model;
pub mod net;
pub mod report;
pub mod school_index;
pub mod sources;
pub mod store;

pub use census::{collect_milesplit, consolidate, CollectOptions, CollectReport};
pub use net::{FetchOptions, Fetcher};
pub use store::{Store, Table};
