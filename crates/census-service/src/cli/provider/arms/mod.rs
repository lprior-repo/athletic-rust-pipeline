//! One function per adapter arm: each builds its adapter's `Options` from the shared
//! [`ProviderArgs`] and returns the report the dispatcher prints.
//!
//! The split is what keeps `provider.rs` under the 300-line budget: the clap surface, the state
//! resolution and the dispatch stay there, and the per-adapter argument marshalling lives here.
//!
//! `meet_sources` carries the arms whose payload is meet-shaped, `association_sources` the ones that
//! return a roster, a coach directory or a state association's own pages. Each shard's functions are
//! re-exported below at this module's own visibility, so a caller sees one module either way.

/// Per-state concurrency for the roster walk, matching the `collect` subcommand's default. Traffic
/// is bounded by the fetcher's per-host gate, not by this number; it only removes idle time.
const DEFAULT_COLLECT_CONCURRENCY: usize = 4;

mod association_sources;
mod meet_sources;

pub(super) use association_sources::{
    ciac_report, coach_contacts_report, ihsa_report, ihsa_tournament_report, ks_report, mpa_report,
    mshsl_report, ohsaa_report, plain_names_report, riil_report, wiaa_report, wiaa_results_report,
};
pub(super) use meet_sources::{
    athleticlive_athletes_report, athleticlive_report, athleticlive_results_report,
    athleticnet_report, milesplit_report, milesplit_results_report, wayzata_report,
};
