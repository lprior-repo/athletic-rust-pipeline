//! Source adapters. Each adapter is a plain async function taking [`AdapterContext`]; there is no
//! trait indirection because adapters differ in shape (single request, paginated index, artifact
//! walk) and the orchestration in [`crate::census`] calls them directly.

//! Shared concurrency bound for bounded fan-out across adapters.
//!
//! This constant is the default maximum number of concurrent async operations
//! (HTTP requests, file walks, pagination batches) that any single adapter
//! may run. Adapters that need a different bound may override it locally.
pub const CONCURRENCY_BOUND: usize = 8;

pub mod athleticlive;
pub mod athleticlive_athletes;
pub mod athleticnet;
pub mod coach_contacts;
pub mod compiled;
pub mod hytek;
pub mod ihsa;
pub mod ks;
pub mod milesplit;
pub mod mshsl;
pub mod ohsaa;
pub mod plain_names;
pub mod raceday;
pub mod result_file;
pub mod wayzata;
pub mod wiaa;
pub mod wiaa_results;
pub mod xc;

use crate::net::{FetchOptions, Fetcher};
use crate::store::{Store, Table};
use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;

/// Shared per-run context handed to every adapter.
pub struct AdapterContext<'a> {
    pub fetcher: &'a Fetcher,
    pub store: &'a Store,
    pub refresh: bool,
    pub school_year: census_domain::model::SchoolYear,
    pub observed_on: String,
}

impl<'a> AdapterContext<'a> {
    pub fn fetch_options(&self) -> FetchOptions {
        FetchOptions {
            refresh: self.refresh,
            allow_not_found: false,
            headers: Vec::new(),
        }
    }
}

/// What an adapter accomplished. Serialized into the journal and into the run report.
#[derive(Debug, Clone, Serialize)]
pub struct AdapterReport {
    pub adapter: String,
    pub rows: u64,
    pub requests: u64,
    pub from_cache: u64,
    pub errors: u64,
    /// Rows that carry a usable contact address (coach adapters only; 0 elsewhere).
    #[serde(default)]
    pub with_email: u64,
    /// Unit label for `rows` (schools, athletes, rosters, …).
    pub unit: String,
    pub notes: Vec<String>,
}

impl AdapterReport {
    pub fn new(adapter: impl Into<String>, unit: impl Into<String>) -> Self {
        Self {
            adapter: adapter.into(),
            rows: 0,
            requests: 0,
            from_cache: 0,
            errors: 0,
            with_email: 0,
            unit: unit.into(),
            notes: Vec::new(),
        }
    }

    pub fn note(&mut self, message: impl Into<String>) {
        self.notes.push(message.into());
    }
}

/// Default per-host request spacing. Bound requires 10 s; everything else starts at 1 s.
pub fn default_host_delays() -> std::collections::HashMap<String, Duration> {
    [
        ("gobound.com", Duration::from_secs(10)),
        ("www.gobound.com", Duration::from_secs(10)),
    ]
    .into_iter()
    .map(|(host, delay)| (host.to_string(), delay))
    .collect()
}

/// Directory a run writes its store into.
pub fn default_store_dir() -> PathBuf {
    PathBuf::from("var/midwest-census")
}

/// Append a batch of entities, tolerating an empty batch.
pub fn append_all<T: Serialize>(store: &Store, table: Table, rows: &[T]) -> Result<()> {
    store.append_many(table, rows)
}
