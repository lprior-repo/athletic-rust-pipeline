#![forbid(unsafe_code)]

pub const CONCURRENCY_BOUND: usize = 8;

pub const FLUSH_UNITS: usize = 64;


#[derive(Debug, thiserror::Error)]
pub enum CrawlError {
    #[error(transparent)]
    Fetch(#[from] crate::net::FetchError),
    #[error("regex {pattern} failed to compile: {source}")]
    RegexInit {
        pattern: &'static str,
        #[source]
        source: regex::Error,
    },
    #[error("json decode failed for {url}: {source}")]
    Decode {
        url: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("schema mismatch for {url}: {detail}")]
    Schema { url: String, detail: String },
    #[error("row for {table} failed to encode: {source}")]
    Encode {
        table: String,
        #[source]
        source: serde_json::Error,
    },
    #[error(transparent)]
    Domain(#[from] census_domain::DomainError),
    #[error(transparent)]
    Store(#[from] census_store::StoreError),
    #[error("arithmetic overflow: {detail}")]
    Arithmetic { detail: String },
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{detail}")]
    Invariant { detail: String },
}

pub type CrawlResult<T> = std::result::Result<T, CrawlError>;

pub mod applicability;
pub mod athlete_observations;
pub mod athleticlive;
pub mod athleticlive_athletes;
pub mod athleticnet;
pub mod ciac;
pub mod coach_contacts;
pub mod compiled;
mod context;
pub mod hytek;
pub mod ihsa;
pub mod ks;
pub mod milesplit;
pub mod mpa;
pub mod mshsl;
pub mod net;
pub mod ohsaa;
pub mod plain_names;
pub mod raceday;
pub mod recording;
pub mod registry;
pub mod result_file;
pub mod riil;
pub mod tfrrs;
pub mod wayzata;
pub mod wiaa;
pub mod wiaa_results;
pub mod xc;

pub use registry::{
    bulk_first, descriptor, descriptors, AccessClass, SourceAdmission, SourceCapabilities,
    SourceDescriptor, TransportKind,
};

pub use athlete_observations::athlete_observations_of;
pub use context::{school_observations_of, AdapterContext};
pub use recording::{Recorded, RecordedBatch, RecordedJournal, Recording};

use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct AdapterReport {
    pub adapter: String,
    pub rows: u64,
    pub requests: u64,
    pub from_cache: u64,
    pub errors: u64,
    #[serde(default)]
    pub with_email: u64,
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

pub fn default_host_delays() -> std::collections::HashMap<String, Duration> {
    [
        ("gobound.com", Duration::from_secs(10)),
        ("www.gobound.com", Duration::from_secs(10)),
    ]
    .into_iter()
    .map(|(host, delay)| (host.to_string(), delay))
    .collect()
}

pub fn default_family_delays() -> std::collections::HashMap<String, Duration> {
    [
        ("milesplit.com", Duration::from_secs(1)),
        ("athletic.net", Duration::from_secs(1)),
    ]
    .into_iter()
    .map(|(family, delay)| (family.to_string(), delay))
    .collect()
}
