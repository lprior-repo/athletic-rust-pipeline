//! The fetch-facing tools: the polite-fetcher probe, the registered state sites, and the report a
//! walk prints for the hosts that refused it.
//!
//! These are offline tools with their own store open, not census stages: the stages that walk a source
//! host for the census live in [`super::gather`].

use anyhow::Result;
use census_domain::UsJurisdiction;
use midwest_census::net::{FetchOptions, Fetcher};
use midwest_census::sources::milesplit::Site;
use midwest_census::store::Store;

use super::{build_fetcher, Cli};

/// List the registered MileSplit state sites.
pub(super) fn run_sites() -> Result<()> {
    for jurisdiction in UsJurisdiction::ALL {
        let site = Site::for_jurisdiction(jurisdiction);
        println!("{}\t{}", site.code(), site.host());
    }
    Ok(())
}

/// Fetch a single URL through the polite fetcher (robots-enforced, cached).
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
    println!(
        "status={} bytes={} from_cache={} sha256={} fetched_at={}",
        outcome.status, outcome.bytes, outcome.from_cache, outcome.sha256, outcome.fetched_at
    );
    println!("url={}", outcome.url);
    Ok(())
}

/// Print one `\t`-separated line per blocked host, named with the kind of condition that stopped the
/// walk (§69).
///
/// Plain text on purpose — the machine-readable copy is the report's own `access_conditions` — and
/// read off the fetcher rather than the report so a walk that failed *after* the block still names
/// the host that refused it. A blocked host is printed even when no kind is attached to it.
pub(super) async fn print_blocked_hosts(fetcher: &Fetcher) {
    let conditions = fetcher.access_conditions().await;
    let now = midwest_census::net::now_iso8601();
    for host in fetcher.blocked_hosts(now.as_str()).await {
        match conditions.iter().find(|condition| condition.host == host) {
            Some(condition) => println!("blocked\t{host}\t{}", condition.kind.slug()),
            None => println!("blocked\t{host}"),
        }
    }
}
