use anyhow::Result;
use census_crawl::milesplit::Site;
use census_crawl::net::{FetchOptions, Fetcher};
use census_domain::UsJurisdiction;
use census_store::Store;

use super::{build_fetcher, Cli};

pub(super) fn run_sites() -> Result<()> {
    for jurisdiction in UsJurisdiction::ALL {
        let site = Site::for_jurisdiction(jurisdiction);
        println!("{}\t{}", site.code(), site.host());
    }
    Ok(())
}

pub(super) async fn run_fetch(cli: &Cli, store: &Store, url: &str, refresh: bool) -> Result<()> {
    let fetcher = build_fetcher(cli, store)?;
    let outcome = fetcher
        .get(
            url,
            &FetchOptions {
                refresh,
                ..Default::default()
            },
        )
        .await?;
    tracing::info!(
        status = outcome.status, bytes = outcome.bytes, from_cache = outcome.from_cache,
        content_digest = %outcome.content_digest, fetched_at = %outcome.fetched_at,
        url = %outcome.url, "source fetched"
    );
    Ok(())
}

pub(super) async fn print_blocked_hosts(fetcher: &Fetcher) {
    let conditions = fetcher.access_conditions().await;
    let now = census_crawl::net::now_iso8601();
    for host in fetcher.blocked_hosts(now.as_str()).await {
        match conditions.iter().find(|condition| condition.host == host) {
            Some(condition) => println!("blocked\t{host}\t{}", condition.kind.slug()),
            None => println!("blocked\t{host}"),
        }
    }
}
