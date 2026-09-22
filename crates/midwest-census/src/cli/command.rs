//! The subcommand enum: clap derives the CLI surface, each variant
//! names the module that owns its dispatch body.

use clap::Subcommand;

use super::census_doc::CensusDocArgs;
use super::cycle::RunArgs;
use super::export_data::ExportDataArgs;
use super::gather::CollectArgs;
use super::merge_coaches::MergeCoachesArgs;
use super::national::{JurisdictionArgs, NationalArgs, NationalReportArgs};
use super::provider::ProviderArgs;
use super::publish::{BestsArgs, WorkbookArgs};
use super::qa_reports::QaReportsArgs;
use super::review::ReviewArgs;
use super::school_names::SchoolNamesArgs;
use super::seal::SealArgs;
use super::store::{BackupArgs, RestoreArgs};
use super::verify::VerifyArgs;
use super::verify_coaches::VerifyCoachesArgs;
use census_domain::UsJurisdiction;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(super) enum Command {
    /// Fetch a single URL through the polite fetcher (robots-enforced, cached).
    Fetch {
        url: String,
        /// Ignore the cache and hit the network.
        #[arg(long)]
        refresh: bool,
    },
    /// List the registered MileSplit state sites.
    Sites,
    /// Fetch (and cache) team indexes for the given states.
    Teams {
        /// Comma-separated state codes (WI,MN,IA,IL,MI,IN,OH,MO,KS,NE,ND,SD, or any other USPS
        /// code). Default: WI.
        #[arg(long, value_delimiter = ',')]
        states: Vec<UsJurisdiction>,
        /// Cover every jurisdiction (50 states + DC). Cannot be combined with `--states`.
        #[arg(long)]
        all_states: bool,
        #[arg(long)]
        refresh: bool,
    },
    /// Enumerate the meets a state's results index publishes, as `source_meets` rows.
    Meets {
        /// Comma-separated state codes (WI,MN,IA,IL,MI,IN,OH,MO,KS,NE,ND,SD, or any other USPS
        /// code). Default: WI.
        #[arg(long, value_delimiter = ',')]
        states: Vec<UsJurisdiction>,
        /// Cover every jurisdiction (50 states + DC). Cannot be combined with `--states`.
        #[arg(long)]
        all_states: bool,
        /// Season start year, the same convention as `--school-year` (2026 = the 2026-27 season).
        #[arg(long, default_value_t = 2026)]
        year: u16,
        /// Ignore caches and re-read every page (robots still enforced).
        #[arg(long)]
        refresh: bool,
    },
    /// Walk rosters and emit canonical entities for the given states.
    Collect(CollectArgs),
    /// Import the researched official coach-contact CSV into canonical entities.
    ImportCoaches {
        /// Path to `data/coach-contacts.csv` (research workspace artifact).
        csv: PathBuf,
        /// ISO date used when a row carries no `last_observed`.
        #[arg(long)]
        observed_on: Option<String>,
    },
    /// Run one association contact adapter by name.
    Provider(ProviderArgs),
    /// Merge append observations into `out/*.jsonl` snapshots.
    Consolidate,
    /// Derive the durable indexes into the store: source-object identities, retained conflicts and
    /// review cases, coverage, and a snapshot of the pass.
    Index,
    /// Compute the measured census from the store.
    Report {
        /// Print the census JSON to stdout as well as writing files.
        #[arg(long)]
        print: bool,
        /// Restrict the census to core evidence: Athletic.net and the AthleticLIVE derivative are
        /// excluded, exactly as they are when those adapters are never registered.
        #[arg(long)]
        core: bool,
    },
    /// Reduce the consolidated tables to one best mark per athlete and event.
    Bests(BestsArgs),
    /// Build the census workbook (`.xlsx`) and its text sidecars.
    Workbook(WorkbookArgs),
    /// Seal the census: assemble §70's evidence, then complete or refuse by name.
    Seal(SealArgs),
    /// Verify workbook data rows against the store.
    Verify(VerifyArgs),
    /// Run the whole cycle in one command: gather the authorized registry, consolidate, publish both
    /// census scopes, reduce best marks, and write the workbook.
    Run(RunArgs),
    /// Print the Fjall store's per-table observation counts and on-disk footprint.
    FjallStats,
    /// Import pre-Fjall JSONL journals into the store (one-time), then print the store stats.
    ImportLegacy,
    /// Back up the store's durable material into a directory.
    StoreBackup(BackupArgs),
    /// Restore a backup into a new directory.
    StoreRestore(RestoreArgs),
    /// Check the store's own integrity.
    StoreIntegrity,
    /// Print the command that runs the `midwest-serve` Restate endpoint.
    Serve,
    /// Submit the durable national census fan-out (one `JurisdictionCensus` per state) and observe
    /// it. Requires a running `midwest-serve` deployment; opens no store of its own.
    National(NationalArgs),
    /// Submit one jurisdiction's durable census and observe it.
    Jurisdiction(JurisdictionArgs),
    /// Print the last report the national run wrote, without starting one.
    NationalReport(NationalReportArgs),
    /// Ask a local model about the review cases the index retained, and record the verdicts whose
    /// proposals the store's own evidence backs.
    Review(ReviewArgs),
    /// Merge and validate coach-lane fragments into one importable CSV.
    MergeCoaches(MergeCoachesArgs),
    /// Re-derive every coach-fragment row from its own cited pages before anything merges: fetches
    /// each citation, keeps only rows whose value and role both re-derive, and writes the verified
    /// fragments, freeze log, audit table, per-row verdicts and a published-artifact reconciliation.
    VerifyCoaches(VerifyCoachesArgs),
    /// Check that every assignment report exists, carries required schema headings, and has
    /// the correct leading header and a Status: line near the top.
    QaReports(QaReportsArgs),
    /// Export the canonical snapshots into CSV data products (canonical-*.csv, seeds, recruiting).
    ExportData(ExportDataArgs),
    /// Extract distinct, trimmed school names for a state from the entities JSONL.
    SchoolNames(SchoolNamesArgs),
    /// Render `synthesis/10-measured-census.md` from the pipeline's snapshots.
    CensusDoc(CensusDocArgs),
}
