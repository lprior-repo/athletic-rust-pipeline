//! `census-crawl` — the acquisition plane: fetching, decoding and mapping provider data onto the
//! census's canonical rows.
//!
//! Each adapter is a plain async function taking [`AdapterContext`]; there is no trait indirection
//! because adapters differ in shape (single request, paginated index, artifact walk). The
//! orchestration that calls them — the sweep, the meet walk, the CLI — is a *consumer* of this crate
//! (`census-service`), which is what keeps a provider's shape from leaking into the run's shape.
//!
//! Two modules carry the crate's shared machinery:
//!
//! * [`net`] — the robots-enforcing, cache-first, per-host-rate-limited fetcher with request
//!   evidence, plus the Restate-backed browser lane for pages a plain fetch cannot render.
//! * [`registry`] — the provider table: which adapters exist, what they claim to cover, and how a
//!   run selects them. It is the crate's outbound edge to nothing else (the net layer reads it to
//!   name the adapter that made a request), which is why it lives here rather than in the run
//!   crate's provider-selection code.
//!
//! The observation funnel is split by half rather than by kind: [`observe_schools_of`] lives with the
//! context that calls it, and [`athlete_observations`] holds the athlete half in its own file, which
//! is what keeps every one of these files inside the repository's source-length budget.

/// Shared concurrency bound for bounded fan-out across adapters.
///
/// This constant is the default maximum number of concurrent async operations
/// (HTTP requests, file walks, pagination batches) that any single adapter
/// may run. Adapters that need a different bound may override it locally.
pub const CONCURRENCY_BOUND: usize = 8;

/// Units one store page covers: an adapter commits the rows a unit produced and the journal entry
/// that names it together, and it commits every this many units.
///
/// The page is the unit of both durability and resumability: a run that stops between two pages
/// re-reads at most one page's units, and no reader can see a journaled unit whose rows are
/// missing.
pub const FLUSH_UNITS: usize = 64;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Adapter-layer failures. Every adapter returns [`CrawlResult`]; `anyhow` is reserved for the
/// CLI shells.
#[derive(Debug, thiserror::Error)]
pub enum CrawlError {
    /// A fetch failed (robots, HTTP status, timeout, transport, cache).
    #[error(transparent)]
    Fetch(#[from] crate::net::FetchError),
    /// A `Regex` static failed to compile.
    #[error("regex {pattern} failed to compile: {source}")]
    RegexInit {
        pattern: &'static str,
        #[source]
        source: regex::Error,
    },
    /// A JSON payload did not decode.
    #[error("json decode failed for {url}: {source}")]
    Decode {
        url: String,
        #[source]
        source: serde_json::Error,
    },
    /// A page's schema did not match the adapter's contract.
    #[error("schema mismatch for {url}: {detail}")]
    Schema { url: String, detail: String },
    /// A row could not be encoded as the wire JSON a recording carries.
    #[error("row for {table} failed to encode: {source}")]
    Encode {
        table: String,
        #[source]
        source: serde_json::Error,
    },
    /// A canonical value could not be formed from parsed input.
    #[error(transparent)]
    Domain(#[from] census_domain::DomainError),
    /// The store rejected an append or scan.
    #[error(transparent)]
    Store(#[from] census_store::StoreError),
    /// Counters, offsets or sizing math overflowed.
    #[error("arithmetic overflow: {detail}")]
    Arithmetic { detail: String },
    /// A local file operation failed.
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// An internal invariant was violated (a bug, not external input).
    ///
    /// Displays the carried message verbatim: golden corpora pin `error.to_string()`.
    #[error("{detail}")]
    Invariant { detail: String },
}

/// Result alias for adapter code.
pub type CrawlResult<T> = std::result::Result<T, CrawlError>;

pub mod applicability;
pub mod athlete_observations;
pub mod athleticlive;
pub mod athleticlive_athletes;
pub mod athleticnet;
pub mod coach_contacts;
pub mod compiled;
mod context;
pub mod hytek;
pub mod ihsa;
pub mod ks;
pub mod milesplit;
pub mod mshsl;
pub mod net;
pub mod ohsaa;
pub mod plain_names;
pub mod raceday;
pub mod recording;
pub mod registry;
pub mod result_file;
pub mod tfrrs;
pub mod wayzata;
pub mod wiaa;
pub mod wiaa_results;
pub mod xc;

// The source capability registry (§10/§11): what each adapter can be asked for, what a request to it
// costs its origin, and the bulk-meet-first ordering a plan starts from.
pub use registry::{
    bulk_first, descriptor, descriptors, AccessClass, SourceAdmission, SourceCapabilities,
    SourceDescriptor, TransportKind,
};

pub use athlete_observations::{observe_athletes_of, stamp_source_athletes};
pub use context::{observe_schools_of, AdapterContext};
pub use recording::{Recorded, RecordedBatch, RecordedJournal, Recording};

use census_store::{Store, Table};
use serde::Serialize;
use std::time::Duration;

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

/// Sources that enforce one ceiling for the whole client, so every host below them shares one
/// budget (§10) instead of one budget per subdomain.
///
/// MileSplit is here because the measured refusal is client-wide: the 2026-09-22 national walk was
/// refused on `tx.milesplit.com` *and* on `wi.milesplit.com` at the same moment, while still being
/// allowed to continue with pages it had already fetched — a per-host model cannot see that, and a
/// fifty-one-state fan-out against it spends fifty-one budgets. Athletic.net is here on the same
/// reasoning: its ranking, profile, meet and result endpoints are one site's traffic and one
/// origin's policy, which is what §10 requires the run to treat them as.
///
/// One second of spacing per family is a deliberate cut below the per-host ceiling: a family is
/// many hosts' worth of demand, and the point of the shared gate is that the *source* sees the rate
/// the policy promises, not the rate any one subdomain could otherwise justify.
pub fn default_family_delays() -> std::collections::HashMap<String, Duration> {
    [
        ("milesplit.com", Duration::from_secs(1)),
        ("athletic.net", Duration::from_secs(1)),
    ]
    .into_iter()
    .map(|(family, delay)| (family.to_string(), delay))
    .collect()
}

/// Append a batch of entities, tolerating an empty batch.
pub fn append_all<T: Serialize>(
    store: &Store,
    table: Table,
    rows: &[T],
) -> census_store::StoreResult<()> {
    store.append_many(table, rows)
}
