mod export;
mod generation;
mod generation_error;
mod geocode;
mod join;
mod manifest;
mod pipeline;
mod preflight;
mod print;
mod publish;
mod read;
mod report;

pub use generation_error::GenerationError;
pub use join::{
    build_lane_evidence, join_generation, parse_source_pairs, process, Counters, JoinError,
    JoinReport, LaneEvidence, LaneSelection, LaneSet, Mode, OutcomeRow, Overrides,
};
pub use manifest::{verify_current, VerifiedGeneration};
pub use report::{
    ChangeReport, CorpusReport, LaneReport, LedgerRow, PhaseReport, Report, SourceDecision,
};

use anyhow::{bail, Context, Result};
use census_domain::school_directory::{ChangeSet, YearMonth};
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
#[command(
    about = "Read the school-directory artifacts into one collapsed corpus, diff it against a baseline, and export it. Opens no store"
)]
pub struct SchoolAddressArgs {
    #[arg(long, help = "NCES CCD school file (the decompressed csv member)")]
    pub ccd: Option<PathBuf>,

    #[arg(long, help = "NCES PSS public-use file (csv)")]
    pub pss: Option<PathBuf>,

    #[arg(
        long = "state-ed-index",
        help = "State education agency school index page; repeat per page"
    )]
    pub state_ed_index: Vec<PathBuf>,

    #[arg(
        long = "state-ed-profile",
        help = "State education agency school profile page; repeat per page"
    )]
    pub state_ed_profile: Vec<PathBuf>,

    #[arg(
        long = "state-ed-tabular",
        help = "State education agency tabular directory; repeat per file"
    )]
    pub state_ed_tabular: Vec<PathBuf>,

    #[arg(long, help = "Private-association membership listing; repeat per file")]
    pub associations: Vec<PathBuf>,

    #[arg(
        long = "association-directory",
        value_name = "SLUG=PATH",
        help = "Athletic-association directory (admitted slugs: tssaa); repeat per file"
    )]
    pub association_directory: Vec<String>,

    #[arg(
        long,
        help = "Generation root: stage and fsync artifacts, rename immutable generations/<digest>, fsync, then atomically swap current; legacy flat layouts are refused"
    )]
    pub out: PathBuf,

    #[arg(
        long,
        help = "Read a verified generation baseline (use <out>/current/baseline.json); absent path starts a first baseline; writes are authoritative inside the new generation"
    )]
    pub baseline: Option<PathBuf>,

    #[arg(
        long,
        help = "Read a verified generation update ledger (use <out>/current/update_ledger.json); writes are authoritative inside the new generation"
    )]
    pub ledger: Option<PathBuf>,

    #[arg(
        long,
        help = "Run month (`YYYY-MM`); required with --baseline or --ledger"
    )]
    pub now: Option<String>,

    #[arg(
        long,
        help = "Geocode the addresses whose entries carry none through the Google client: needs GOOGLE_MAPS_API_KEY or GOOGLE_API_KEY, stamps the filled coordinates as the `geocoder` source and rewrites nothing the artifacts already publish"
    )]
    pub geocode: bool,

    #[arg(
        long = "validate-postal",
        help = "Validate the addresses that carry a ZIP through the USPS client: needs USPS_API_TOKEN and only counts the verdicts, because rewriting a published field with a weaker source would contradict its provenance"
    )]
    pub validate_postal: bool,
}

pub fn run(args: &SchoolAddressArgs) -> Result<()> {
    let now = run_month(args)?;
    preflight::check(args)?;
    let lanes = read::lanes(args)?;
    let mut corpus = pipeline::collapse(&lanes);
    let phases = geocode::apply(args, &mut corpus)?;
    let changes = pipeline::diff(args, &corpus.entries)?;
    let (ledger, schedule) = pipeline::schedule(args, &lanes, now)?;
    let report = build_report(&lanes, &corpus, changes.as_ref(), &schedule, phases, now);
    let outputs = publish::outputs(args, &corpus, changes.as_ref(), &ledger, report)?;
    print::report(
        &lanes,
        &corpus,
        changes.as_ref(),
        &schedule,
        phases,
        &outputs,
    );
    Ok(())
}

fn build_report(
    lanes: &[read::LaneRead],
    corpus: &pipeline::Corpus,
    changes: Option<&ChangeSet>,
    schedule: &[SourceDecision],
    phases: Option<PhaseReport>,
    now: Option<YearMonth>,
) -> Report {
    Report {
        manifest_digest: String::new(),
        now: now.map(|value| value.to_string()),
        lanes: lanes.iter().map(|lane| lane.report.clone()).collect(),
        corpus: CorpusReport {
            rows: corpus.rows,
            entries: corpus.entries.len(),
            skipped: corpus.skipped,
            notes: corpus.notes,
            merges: corpus.merges,
        },
        changes: changes.map(ChangeReport::of),
        schedule: schedule.to_vec(),
        outputs: Vec::new(),
        phases,
    }
}

fn run_month(args: &SchoolAddressArgs) -> Result<Option<YearMonth>> {
    let now = args
        .now
        .as_ref()
        .map(|text| {
            text.parse::<YearMonth>()
                .with_context(|| format!("--now {text} is not a `YYYY-MM` month"))
        })
        .transpose()?;
    if now.is_none() && (args.baseline.is_some() || args.ledger.is_some()) {
        bail!(
            "--baseline and --ledger diff against a run month: pass `--now YYYY-MM` (this verb \
             reads no process clock, so the month is the caller's)"
        );
    }
    Ok(now)
}
