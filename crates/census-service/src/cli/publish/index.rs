use anyhow::{Context, Result};
use census_store::Store;
use clap::Args;

use crate::cli::contact_school_year;

#[derive(Args, Debug)]
pub(crate) struct IndexArgs {
    #[arg(
        help = "School year the contact tenure and coach cells are assessed against (2026 = the 2026-27 school year)"
    )]
    #[arg(long)]
    school_year: u16,
}

pub(crate) fn run_index(store: &Store, args: &IndexArgs) -> Result<()> {
    let finished_on = census_crawl::net::today_iso();
    let school_year = contact_school_year(args.school_year)?;
    let report = census_reconcile::index::derive(store, "index", &finished_on, school_year)
        .context("deriving the durable indexes")?;
    println!(
        "index\tsource_identities={} conflicts={} reviews={} superseded={} coverage={} snapshots={}",
        report.source_identities,
        report.conflicts,
        report.reviews,
        report.superseded,
        report.coverage,
        report.snapshots
    );
    Ok(())
}
