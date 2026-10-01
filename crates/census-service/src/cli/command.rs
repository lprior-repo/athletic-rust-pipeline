use clap::Subcommand;

use super::browser_session::BrowserSessionArgs;
use super::census_doc::CensusDocArgs;
use super::cycle::RunArgs;
use super::export_data::ExportDataArgs;
use super::gather::{CollectArgs, MeetsArgs, TeamsArgs};
use super::merge_coaches::MergeCoachesArgs;
use super::national::{JurisdictionArgs, NationalArgs, NationalReportArgs};
use super::open_work::OpenWorkArgs;
use super::provider::ProviderArgs;
use super::publish::{BestsArgs, ReportArgs, WorkbookArgs};
use super::qa_reports::QaReportsArgs;
use super::review::ReviewArgs;
use super::school_names::SchoolNamesArgs;
use super::seal::SealArgs;
use super::store::{BackupArgs, RestoreArgs};
use super::verify::VerifyArgs;
use super::verify_coaches::VerifyCoachesArgs;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(super) enum Command {
    #[command(
        about = "Fetch a single URL through the polite fetcher (paced at the registry rate, cached)"
    )]
    Fetch {
        url: String,
        #[arg(help = "Ignore the cache and hit the network")]
        #[arg(long)]
        refresh: bool,
    },
    #[command(about = "List the registered MileSplit state sites")]
    Sites,
    #[command(about = "Fetch (and cache) team indexes for the given states")]
    Teams(TeamsArgs),
    #[command(
        about = "Enumerate the meets a state's results index publishes, as `source_meets` rows"
    )]
    Meets(MeetsArgs),
    #[command(about = "Walk rosters and emit canonical entities for the given states")]
    Collect(CollectArgs),
    #[command(about = "Import the researched official coach-contact CSV into canonical entities")]
    ImportCoaches {
        #[arg(help = "Path to `data/coach-contacts.csv` (research workspace artifact)")]
        csv: PathBuf,
        #[arg(help = "ISO date used when a row carries no `last_observed`")]
        #[arg(long)]
        observed_on: Option<String>,
    },
    #[command(about = "Run one association contact adapter by name")]
    Provider(ProviderArgs),
    #[command(about = "Merge append observations into `out/*.jsonl` snapshots")]
    Consolidate,
    #[command(
        about = "Derive the durable indexes into the store: source-object identities, retained conflicts and review cases, coverage, and a snapshot of the pass"
    )]
    Index,
    #[command(about = "Compute the measured census from the store")]
    Report(ReportArgs),
    #[command(about = "Reduce the consolidated tables to one best mark per athlete and event")]
    Bests(BestsArgs),
    #[command(about = "Build the census workbook (`.xlsx`) and its text sidecars")]
    Workbook(WorkbookArgs),
    #[command(about = "Seal the census: assemble §70's evidence, then complete or refuse by name")]
    Seal(SealArgs),
    #[command(
        about = "Verify the complete frozen workbook and sidecar generation without opening the store"
    )]
    Verify(VerifyArgs),
    #[command(
        about = "Run the whole cycle in one command: gather the authorized registry, consolidate, publish both census scopes, reduce best marks, and write the workbook"
    )]
    Run(RunArgs),
    #[command(
        about = "Print the Fjall store's per-table observation counts and on-disk footprint"
    )]
    FjallStats,
    #[command(about = "Back up the store's durable material into a directory")]
    StoreBackup(BackupArgs),
    #[command(about = "Restore a backup into a new directory")]
    StoreRestore(RestoreArgs),
    #[command(about = "Check the store's own integrity")]
    StoreIntegrity,
    #[command(about = "Print the command that runs the `census-serve` Restate endpoint")]
    Serve,
    #[command(
        about = "Submit the durable national census fan-out (one `JurisdictionCensus` per state) and observe it. Requires a running `census-serve` deployment; opens no store of its own"
    )]
    National(NationalArgs),
    #[command(about = "Submit one jurisdiction's durable census and observe it")]
    Jurisdiction(JurisdictionArgs),
    #[command(about = "Print the last report the national run wrote, without starting one")]
    NationalReport(NationalReportArgs),
    #[command(
        about = "Ask a local model about the review cases the index retained, and record the verdicts whose proposals the store's own evidence backs"
    )]
    Review(ReviewArgs),
    #[command(about = "Merge and validate coach-lane fragments into one importable CSV")]
    MergeCoaches(MergeCoachesArgs),
    #[command(
        about = "Re-derive every coach-fragment row from its own cited pages before anything merges: fetches each citation, keeps only rows whose value and role both re-derive, and writes the verified fragments, freeze log, audit table, per-row verdicts and a published-artifact reconciliation"
    )]
    VerifyCoaches(VerifyCoachesArgs),
    #[command(
        about = "Check that every assignment report exists, carries required schema headings, and has the correct leading header and a Status: line near the top"
    )]
    QaReports(QaReportsArgs),
    #[command(
        about = "Export the canonical snapshots into CSV data products (canonical-*.csv, seeds, recruiting)"
    )]
    ExportData(ExportDataArgs),
    #[command(
        about = "Extract distinct, trimmed school names for a state from the entities JSONL"
    )]
    SchoolNames(SchoolNamesArgs),
    #[command(about = "Render `synthesis/10-measured-census.md` from the pipeline's snapshots")]
    CensusDoc(CensusDocArgs),
    #[command(
        about = "Read the durable run's open work: the jurisdiction sweeps that still owe stages, and the source objects that have accepted nothing. Reads the running service, never the store"
    )]
    OpenWork(OpenWorkArgs),
    #[command(
        about = "Drive the headed browser lane the `census-serve` endpoint owns: `start` launches the one persistent profile, `status` reads it, `stop` drains it, and `fetch` posts one request through it. Requires an endpoint started with `--browser-profile`; opens no store"
    )]
    BrowserSession(BrowserSessionArgs),
}
