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
//! * [`store`] — Fjall-backed observation store: append-only observations per entity, merged at
//!   read time, with materialized JSONL snapshots for the read model.
//! * [`sources`] — one module per provider (MileSplit first, then associations/timers).
//! * [`census`] — resumable orchestration.
//! * [`report`] — measured census output (`report.json`, per-state CSV).
//! * [`bests`] — per-athlete best marks reduced from the consolidated performance table.
//! * [`workbook`] — the census as one spreadsheet.
//! * [`restate_services`] — durable Restate services over the same adapters (survive crashes, retry
//!   per step, resume from the journal).
//! * [`bootstrap`] — the service supervisor: task region, cancel/drain/finalize shutdown, drain
//!   report.

#![forbid(unsafe_code)]

pub mod bests;
pub mod bootstrap;
pub mod census;
pub mod model;
pub mod net;
pub mod report;
pub mod restate_services;
pub mod school_index;
pub mod sources;
pub mod store;
pub mod workbook;

pub use census::{collect_milesplit, consolidate, CollectOptions, CollectReport};
pub use net::{FetchOptions, Fetcher};
pub use store::{Store, Table};
