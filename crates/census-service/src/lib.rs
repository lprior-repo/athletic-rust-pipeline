//! `census-service` — independent Midwest high-school TF/XC recruiting census.
//!
//! Goal: build the recruiting graph **without** broad Athletic.net crawling. Athletic.net stays the
//! deepest performance source, but it is entered only through ids obtained from cheaper sources
//! (MileSplit rosters, state associations, timing providers), and only for athletes that another
//! source has already identified.
//!
//! Layers:
//!
//! * [`census_domain::model`] — canonical entities with locally minted deterministic ids, grad-year
//!   cohorts and `ObservedGrade` evidence instead of mutable grade fields.
//! * [`census_crawl`] — the acquisition plane: robots-enforcing, cache-first fetcher, one module per
//!   provider, and the provider registry the run selects from.
//! * [`census_store`] — Fjall-backed observation store: append-only observations per entity, merged at
//!   read time, with materialized JSONL snapshots for the read model.
//! * [`census`] — resumable orchestration.
//! * [`index`] — the durable derived indexes: source-object identities, retained conflicts and
//!   review cases, coverage, and one snapshot per pass.
//! * [`restate_services`] — durable Restate services over the same adapters (survive crashes, retry
//!   per step, resume from the journal).
//! * [`spawn`] — the region-owned task spawner: every task a region starts joins back through one
//!   set, and a drain reaps it inside a deadline instead of leaving it orphaned.
//! * [`bootstrap`] — the service supervisor: task region, cancel/drain/finalize shutdown, drain
//!   report.

#![forbid(unsafe_code)]

pub mod bootstrap;
pub mod census;
pub mod coachverify;
pub mod index;
pub mod ingress;
pub mod outcome;
pub mod restate_services;
pub mod spawn;

pub use census::{collect_milesplit, consolidate, CollectOptions, CollectReport};
