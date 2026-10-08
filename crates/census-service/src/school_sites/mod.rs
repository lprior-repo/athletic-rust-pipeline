mod artifacts;
mod contacts;
mod execution;
mod owners;
mod queue;
pub use execution::run;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use artifacts::{Failure, Report};
pub use contacts::{contact_rows, LaneRow};
pub use queue::{host_roots, PlannedSite, QueueStats};

use anyhow::{Context, Result};
use census_domain::UsJurisdiction;
use clap::Args;
use std::path::{Path, PathBuf};

#[derive(Debug, Args)]
#[command(
    about = "Acquire canonical school-office and athletics-office mailboxes and contact research for source-discovered schools selected by an exact website queue"
)]
pub struct SchoolSitesArgs {
    #[arg(
        help = "Queue JSONL holding {\"state\",\"name\",\"website\"} selectors for existing source-discovered canonical schools; never creates school population"
    )]
    pub input: PathBuf,

    #[arg(
        long,
        value_name = "DIR",
        help = "Report directory; defaults to <store>/out/school-sites"
    )]
    pub out: Option<PathBuf>,

    #[arg(
        long,
        value_name = "CODE",
        help = "Jurisdiction for records that carry none (code or name)"
    )]
    pub state: Option<String>,

    #[arg(
        long,
        help = "Acquire at most this many queue-selected schools, after filtering"
    )]
    pub limit: Option<usize>,

    #[arg(
        long,
        help = "Evenly spaced sample of this many queue-selected schools, before --limit"
    )]
    pub sample: Option<usize>,

    #[arg(
        long,
        help = "Refetch physical source captures instead of using the HTTP cache; artifact presence never certifies canonical completion"
    )]
    pub refresh: bool,

    #[arg(
        long,
        help = "Treat every host in the queue as authorized for this collection",
        long_help = "Authorize commissioned queue hosts for acquisition under existing robots and pacing policy. This grant never qualifies a canonical school owner, substitutes a website, or proves a redirected page belongs to the selected school."
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
