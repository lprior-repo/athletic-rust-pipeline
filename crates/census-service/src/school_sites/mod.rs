mod artifacts;
mod contacts;
mod queue;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use artifacts::{Failure, Report};
pub use contacts::{contact_rows, LaneRow};
pub use queue::{host_roots, PlannedSite, QueueStats};

use anyhow::{Context, Result};
use census_crawl::net::Fetcher;
use census_crawl::school_sites::{crawl_site, parse_queue, Rules, SiteOutcome};
use census_crawl::CONCURRENCY_BOUND;
use census_domain::UsJurisdiction;
use clap::Args;
use futures::stream::{self, StreamExt};
use std::path::{Path, PathBuf};

#[derive(Debug, Args)]
#[command(
    about = "Crawl school athletics sites from a school-website queue, writing per-site JSON evidence and fragment CSVs for verify-coaches"
)]
pub struct SchoolSitesArgs {
    #[arg(help = "Queue JSONL holding one {\"state\",\"name\",\"website\"} record per school")]
    pub input: PathBuf,

    #[arg(
        long,
        value_name = "DIR",
        help = "Artifact directory; defaults to <store>/out/school-sites"
    )]
    pub out: Option<PathBuf>,

    #[arg(
        long,
        value_name = "CODE",
        help = "Jurisdiction for records that carry none (code or name)"
    )]
    pub state: Option<String>,

    #[arg(long, help = "Crawl at most this many sites, after filtering")]
    pub limit: Option<usize>,

    #[arg(long, help = "Evenly spaced sample of this many sites, before --limit")]
    pub sample: Option<usize>,

    #[arg(
        long,
        help = "Refetch pages and rewrite artifacts that already exist, instead of resuming"
    )]
    pub refresh: bool,

    #[arg(
        long,
        help = "Treat every host in the queue as authorized for this collection",
        long_help = "Treat every host in the queue as authorized for this collection.\n\nSchool sites routinely redirect to another origin (http to https, apex to www, a district host to a platform host). A redirect that stays on a queue host is admitted instead of being refused as an origin outside the registry, and every host is paced no faster than the 2 rps per-host ceiling. This is the operator's statement that these pages were commissioned."
    )]
    pub authorize_queue_hosts: bool,
}

impl SchoolSitesArgs {
    pub fn out_dir(&self, store_root: &Path) -> PathBuf {
        match self.out.clone() {
            Some(out) => out,
            None => store_root.join("out").join("school-sites"),
        }
    }

    fn state_override(&self) -> Result<Option<UsJurisdiction>> {
        match self.state.as_deref() {
            Some(raw) => {
                let state = UsJurisdiction::parse(raw)
                    .with_context(|| format!("--state {raw} is not a US jurisdiction"))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }
}

pub async fn run(fetcher: &Fetcher, args: &SchoolSitesArgs, store_root: &Path) -> Result<Report> {
    let out_dir = args.out_dir(store_root);
    let text = std::fs::read_to_string(&args.input)
        .with_context(|| format!("reading queue {}", args.input.display()))?;
    let (rows, rejected) = parse_queue(&text);
    let mut stats = QueueStats {
        rejected,
        ..QueueStats::default()
    };
    let sites = queue::plan(
        rows,
        args.state_override()?,
        args.sample,
        args.limit,
        &out_dir,
        &mut stats,
    );
    let rules = Rules::new().context("compiling school-site extraction rules")?;
    std::fs::create_dir_all(&out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let fragments_dir = out_dir.join("fragments");
    let site_rows_dir = out_dir.join("site-rows");
    std::fs::create_dir_all(&site_rows_dir)
        .with_context(|| format!("creating {}", site_rows_dir.display()))?;
    let planned = sites.len();

    let mut skipped = 0usize;
    let mut pending: Vec<PlannedSite> = Vec::new();
    for site in sites {
        if queue::resumable(&site, &site_rows_dir, args.refresh) {
            skipped = skipped.saturating_add(1);
        } else {
            pending.push(site);
        }
    }

    let mut crawl_results: Vec<(PlannedSite, Result<SiteOutcome, String>)> = Vec::new();
    let mut stream = stream::iter(pending.into_iter().map(|site| {
        let rules = &rules;
        async move {
            let outcome = crawl_site(rules, fetcher, &site.row, args.refresh).await;
            let outcome = match outcome {
                Ok(value) => Ok(value),
                Err(error) => Err(error.to_string()),
            };
            (site, outcome)
        }
    }))
    .buffer_unordered(CONCURRENCY_BOUND);
    while let Some((site, outcome)) = stream.next().await {
        crawl_results.push((site, outcome));
    }

    crawl_results.sort_by_key(|entry| entry.0.sort_key());

    let mut written = 0usize;
    let mut empty = 0usize;
    let mut emails = 0usize;
    let mut requests = 0usize;
    let mut errors = 0usize;
    let mut failures: Vec<Failure> = Vec::new();
    let mut lane_rows: Vec<LaneRow> = Vec::new();
    for (site, outcome) in &crawl_results {
        match outcome {
            Ok(outcome) => {
                requests = requests.saturating_add(outcome.requests);
                errors = errors.saturating_add(outcome.errors);
                let Some(note) = outcome.note.as_deref() else {
                    let record = artifacts::site_record(site, outcome);
                    let rows: Vec<LaneRow> = if outcome.signals.has_signals() {
                        contacts::contact_rows(site, outcome)
                    } else {
                        empty = empty.saturating_add(1);
                        Vec::new()
                    };
                    let rows_path = site.rows_path(&site_rows_dir);
                    artifacts::write_site_rows(&rows_path, &rows)?;
                    artifacts::write_site(&site.artifact, &record)?;
                    written = written.saturating_add(1);
                    emails = emails.saturating_add(outcome.signals.emails.len());
                    lane_rows.extend(rows);
                    continue;
                };
                failures.push(Failure {
                    state: site.state.code().to_string(),
                    school: site.school.clone(),
                    website: site.website.clone(),
                    detail: note.to_string(),
                });
            }
            Err(detail) => failures.push(Failure {
                state: site.state.code().to_string(),
                school: site.school.clone(),
                website: site.website.clone(),
                detail: detail.clone(),
            }),
        }
    }
    lane_rows.sort();

    let fragment_files = artifacts::publish_state_fragments(&fragments_dir, &site_rows_dir)?;

    let coach_contacts = lane_rows
        .iter()
        .filter(|row| !row.coach_name.is_empty())
        .count();
    let ad_contacts = lane_rows
        .iter()
        .filter(|row| !row.ad_name.is_empty())
        .count();
    let report = Report {
        queue: stats,
        out_dir: out_dir.display().to_string(),
        fragments: fragment_files.join(","),
        planned,
        crawled: written,
        empty,
        skipped,
        failed: failures.len(),
        emails,
        coach_contacts,
        ad_contacts,
        requests,
        errors,
        failures,
    };
    artifacts::write_report(&out_dir.join("report.json"), &report)?;
    Ok(report)
}

pub fn print_report(report: &Report) {
    println!(
        "school-sites\tplanned={} crawled={} empty={} skipped={} failed={}",
        report.planned, report.crawled, report.empty, report.skipped, report.failed
    );
    println!(
        "queue\tread={} rejected={} without_website={} unmapped_state={} duplicate={}",
        report.queue.read,
        report.queue.rejected,
        report.queue.without_website,
        report.queue.unmapped_state,
        report.queue.duplicate
    );
    println!(
        "signals\temails={} coach_contacts={} ad_contacts={} requests={} errors={}",
        report.emails, report.coach_contacts, report.ad_contacts, report.requests, report.errors
    );
    println!("name={}", report.out_dir);
    for failure in &report.failures {
        println!(
            "failed\t{}\t{}\t{}",
            failure.state, failure.school, failure.detail
        );
    }
}
