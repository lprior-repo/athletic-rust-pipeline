#![forbid(unsafe_code)]

#[cfg(test)]
#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

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
    #[error("row for {table} failed to canonicalize: {source}")]
    Canonical {
        table: String,
        #[source]
        source: census_domain::model::CanonicalJsonError,
    },
    #[error(transparent)]
    Domain(#[from] census_domain::DomainError),
    #[error(transparent)]
    Directory(#[from] census_domain::school_directory::DirectoryError),
    #[error(transparent)]
    Store(#[from] census_store::StoreError),
    #[error("arithmetic overflow: {detail}")]
    Arithmetic { detail: String },
    #[error("{resource} reservation of {requested} exceeds limit {limit}")]
    Resource {
        resource: &'static str,
        requested: usize,
        limit: usize,
    },
    #[error(transparent)]
    EventIdentity(#[from] census_domain::model::EventIdentityError),
    #[error(transparent)]
    Specification(#[from] census_domain::model::SpecificationError),
    #[error(transparent)]
    Performance(#[from] census_domain::model::PerformanceError),
    #[error("published performance date {published:?} cannot be compared to snapshot {as_of}")]
    PerformanceDateUnknown {
        published: String,
        as_of: chrono::NaiveDate,
    },
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{detail}")]
    Invariant { detail: String },
    #[error("{path} is not a usable directory artifact: {detail}")]
    DirectoryArtifact {
        path: std::path::PathBuf,
        detail: String,
    },
}

pub type CrawlResult<T> = std::result::Result<T, CrawlError>;

impl CrawlError {
    pub fn retryable(&self) -> bool {
        matches!(self, Self::Fetch(error) if error.retryable())
    }
}

pub mod aia;
pub mod applicability;
pub mod arbiter;
pub mod athlete_observations;
pub mod athleticlive;
pub mod athleticlive_athletes;
pub mod athleticnet;
pub mod bound;
pub mod chsaa;
pub mod ciac;
pub mod coach_contacts;
pub mod coach_directories;
mod cohort;
mod disposition;
pub use disposition::CollectionDisposition;
pub mod compiled;
mod context;
pub mod directory;
pub mod geocode;
pub mod home_campus;
pub mod hytek;
pub mod ihsa;
pub mod ingress;
pub mod ks;
pub mod milesplit;
pub mod mpa;
pub mod mshsl;
pub mod nces;
pub mod net;
pub mod ohsaa;
pub mod pa_piaa;
pub mod plain_names;
pub mod private_assoc;
pub mod raceday;
pub mod recording;
pub mod registry;
pub mod result_file;
mod result_status;
pub mod riil;
pub mod row_hygiene;
pub mod school_sites;
pub mod sidearm_staff;
pub mod state_ed;
pub mod tfrrs;
pub mod tssaa;
pub mod uhsaa;
pub mod wayzata;
pub mod wiaa;
pub mod wiaa_results;
pub mod wikidata;
pub mod xc;

pub use registry::{
    bulk_first, descriptor, descriptors, AccessClass, SourceAdmission, SourceCapabilities,
    SourceDescriptor, TransportKind,
};

pub use athlete_observations::athlete_observations_of;
pub use context::{
    assess_performance_date, school_observations_of, AdapterContext, PerformanceDateAssessment,
};
pub use recording::{PreparedRecorded, Recorded, RecordedBatch, RecordedJournal, Recording};

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnresolvedCounters {
    pub rows: u64,
    pub labels: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionCounters {
    pub rows: u64,
    pub resolved: u64,
    pub unresolved: u64,
    pub retained: u64,
    pub quarantined: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterReport {
    pub adapter: String,
    pub rows: u64,
    pub requests: u64,
    pub from_cache: u64,
    pub errors: u64,
    #[serde(default)]
    pub with_email: u64,
    #[serde(default)]
    pub rejections: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved: Option<UnresolvedCounters>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<ResolutionCounters>,
    pub unit: String,
    pub notes: Vec<String>,
    #[serde(default)]
    pub disposition: CollectionDisposition,
    #[serde(default)]
    pub unfinished: Vec<String>,
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
            rejections: 0,
            unresolved: None,
            resolution: None,
            unit: unit.into(),
            notes: Vec::new(),
            disposition: CollectionDisposition::Unknown,
            unfinished: Vec::new(),
        }
    }

    pub fn finish_frontier(&mut self) {
        self.disposition = if self.errors == 0
            && self.rejections == 0
            && self
                .unresolved
                .is_none_or(|value| value.rows == 0 && value.labels == 0)
            && self.resolution.is_none_or(|value| value.unresolved == 0)
            && self.unfinished.is_empty()
        {
            CollectionDisposition::Complete
        } else {
            CollectionDisposition::Partial
        };
    }

    pub fn note(&mut self, message: impl Into<String>) {
        self.notes.push(message.into());
    }

    pub fn reject(&mut self, message: impl Into<String>) {
        self.rejections = self.rejections.saturating_add(1);
        self.note(message);
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
