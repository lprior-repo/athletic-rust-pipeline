//! The `verify-coaches` subcommand: re-derive every coach-fragment row from its own cited pages.
//!
//! This is the gate between a research lane's claim and the store: fragments go in, verified
//! fragments (plus the freeze log, audit table, per-row verdict CSV and a reconciliation against a
//! published artifact) come out. Only rows whose value *and* role re-derive from the fetched bytes
//! reach `--out`; `merge-coaches` then merges that directory and nothing else.

mod write;

use anyhow::{bail, Result};
use clap::Args;
use futures::stream::{self, StreamExt};
use midwest_census::coachverify::{self, FragmentOutcome, GateOptions};
use midwest_census::net::Fetcher;
use std::path::PathBuf;
use std::time::Duration;

/// Fragments verified concurrently. The fetcher still paces per host, so this only widens the set
/// of hosts in flight.
const DEFAULT_JOBS: usize = 8;

#[derive(Args, Debug)]
pub(super) struct VerifyCoachesArgs {
    /// Fragment CSVs, or directories containing them (comma-separated).
    #[arg(long, value_delimiter = ',', value_name = "PATH", required = true)]
    pub(super) fragments: Vec<PathBuf>,

    /// HTTP cache directory — the store's cache by default, so gate fetches and pipeline fetches
    /// share one cache. The store itself is never opened: the gate reads and writes files.
    #[arg(long, value_name = "DIR", default_value = "var/midwest-census/http")]
    pub(super) cache_dir: PathBuf,

    /// Default per-host delay between requests, milliseconds.
    #[arg(long, default_value_t = 1000)]
    pub(super) delay_ms: u64,

    /// User agent the gate identifies with.
    #[arg(long, value_name = "UA")]
    pub(super) user_agent: Option<String>,

    /// Host to treat as authorized (repeatable).
    #[arg(long = "authorized-host", value_name = "HOST")]
    pub(super) authorized_hosts: Vec<String>,

    /// Directory that receives one verified fragment (same file name) per input.
    #[arg(long, value_name = "DIR")]
    pub(super) out: PathBuf,

    /// Directory that receives one `<ST>.csv` per state — the shape `merge-coaches` consumes.
    #[arg(long, value_name = "DIR")]
    pub(super) union: Option<PathBuf>,

    /// Freeze manifest: timestamp, one sha256 line per input fragment, then the verdict lines.
    #[arg(long, value_name = "PATH")]
    pub(super) manifest: Option<PathBuf>,

    /// Freeze log: the tally line per fragment, appended.
    #[arg(long, value_name = "LOG")]
    pub(super) log: Option<PathBuf>,

    /// Markdown audit table for the fragments just verified.
    #[arg(long, value_name = "MD")]
    pub(super) report: Option<PathBuf>,

    /// Per-row verdict CSV (fragment, identity cells, citation, verdict).
    #[arg(long, value_name = "CSV")]
    pub(super) csv: Option<PathBuf>,

    /// Published merged CSV to reconcile against the verified rows.
    #[arg(long, value_name = "CSV")]
    pub(super) reconcile: Option<PathBuf>,

    /// Fragments verified concurrently.
    #[arg(long, default_value_t = DEFAULT_JOBS)]
    pub(super) jobs: usize,

    /// Skip the XHR + Referer pass.
    #[arg(long)]
    pub(super) no_xhr: bool,

    /// Skip the NSAA export-screen form POST pass.
    #[arg(long)]
    pub(super) no_nsaa_post: bool,

    /// Treat every host the fragments cite as authorized for this collection.
    ///
    /// robots.txt is still read and cached, but a disallowed path on a cited host is counted as
    /// `robots_authorized` instead of blocking the request, which is the operator's statement that
    /// these pages were commissioned. Without it the gate refuses those pages and the rows citing
    /// them cannot ship.
    #[arg(long)]
    pub(super) authorize_cited_hosts: bool,

    /// `pdftotext` binary for PDF citations (pass an empty string to disable inflation).
    #[arg(long, value_name = "BIN")]
    pub(super) pdftotext: Option<String>,

    /// Ignore cached bodies and hit the network (robots still enforced).
    #[arg(long)]
    pub(super) refresh: bool,
}

pub(super) async fn run_verify_coaches(args: &VerifyCoachesArgs) -> Result<()> {
    let files = write::collect_fragments(&args.fragments)?;
    if files.is_empty() {
        bail!("no fragment CSVs found under {:?}", args.fragments);
    }
    let pdftotext = match args.pdftotext.as_deref() {
        None => Some("pdftotext".to_string()),
        Some("") => None,
        Some(binary) => Some(binary.to_string()),
    };
    let options = GateOptions {
        xhr_pass: !args.no_xhr,
        nsaa_post: !args.no_nsaa_post,
        pdftotext,
        refresh: args.refresh,
    };
    let mut authorized = args.authorized_hosts.clone();
    if args.authorize_cited_hosts {
        let cited = coachverify::cited_hosts(&files)?;
        println!("authorizing {} cited hosts", cited.len());
        authorized.extend(cited);
    }
    authorized.retain(|host| !host.trim().is_empty());
    authorized.sort();
    authorized.dedup();
    let fetcher = Fetcher::new(
        &args.cache_dir,
        args.user_agent.clone(),
        Duration::from_millis(args.delay_ms),
        midwest_census::sources::default_host_delays(),
        authorized,
    )?
    .with_family_budgets(midwest_census::sources::default_family_delays());
    let out_dir = args.out.clone();
    let jobs = args.jobs.max(1);
    let results: Vec<Result<FragmentOutcome>> = stream::iter(files.iter().cloned())
        .map(|path| {
            let fetcher = &fetcher;
            let out_dir = out_dir.as_path();
            let options = &options;
            async move { coachverify::verify_fragment(fetcher, &path, out_dir, options).await }
        })
        .buffer_unordered(jobs)
        .collect()
        .await;
    let mut outcomes = Vec::with_capacity(results.len());
    let mut failures = Vec::new();
    for result in results {
        match result {
            Ok(outcome) => outcomes.push(outcome),
            Err(error) => failures.push(error),
        }
    }
    outcomes.sort_by(|left, right| left.file.cmp(&right.file));
    write::print_results(args, &outcomes, &files, &fetcher, &failures, &out_dir).await
}
