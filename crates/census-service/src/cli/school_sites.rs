use super::Cli;
use anyhow::{Context, Result};
use census_crawl::school_sites::SOURCE_ID;
use census_store::Store;

pub(super) async fn run(
    cli: &Cli,
    args: &census_service::school_sites::SchoolSitesArgs,
) -> Result<()> {
    let store_root = cli.store_root();
    let store = Store::open(&store_root).context("opening the store that owns the fetch cache")?;
    let hosts = if args.authorize_queue_hosts {
        census_service::school_sites::host_roots(&args.input)?
    } else {
        Vec::new()
    };
    let fetcher = super::build_fetcher_authorizing(cli, &store, hosts)?.with_source(SOURCE_ID);
    let report = census_service::school_sites::run(&fetcher, args, &store).await?;
    census_service::school_sites::print_report(&report);
    if report.failed > 0 || report.errors > 0 {
        anyhow::bail!(
            "school contact acquisition remains owed: {} failed subjects, {} acquisition errors",
            report.failed,
            report.errors
        );
    }
    Ok(())
}
