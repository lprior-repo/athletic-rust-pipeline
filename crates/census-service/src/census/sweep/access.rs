//! The §69 access-block stop: what a source refusing this client means for a walk, and the rows the
//! run leaves behind for the next one.
//!
//! Split out of the parent module when the stop outgrew the repository's 300-line budget: `sweep.rs`
//! keeps the walk, this module keeps the condition that ends it and the access rows it records.

use tracing::warn;

use census_crawl::net::{FetchError, Fetcher};
use census_crawl::CrawlError;
use census_domain::model::SourceAccessCondition;
use census_store::{Store, Table};

/// Whether one failure is a hard access block: 403 (refused outright) or 429 (told to back off).
///
/// Both statuses reach a caller inside [`FetchError::Http`]: the retry loop classifies 429 as
/// transient and, once the attempt budget is spent, reports the status it kept seeing. The status —
/// not the variant — is therefore what a lane stops on.
pub(super) fn refused(error: &CrawlError) -> bool {
    match error {
        CrawlError::Fetch(FetchError::Http { status, .. }) => *status == 403 || *status == 429,
        // A rate limit the fetch layer classified before a status was attached to it.
        CrawlError::Fetch(FetchError::RateLimited { .. }) => true,
        _ => false,
    }
}

/// What one run observed about access, and what came of trying to persist it.
pub(super) struct Observed {
    /// Every access condition this run observed, sorted by row id.
    pub(super) conditions: Vec<SourceAccessCondition>,
    /// Hosts whose condition still blocks work: non-empty means the walk stopped short of complete.
    pub(super) blocked_hosts: Vec<String>,
    /// `source_access` writes that failed, for the report's error count.
    pub(super) failures: u64,
}

/// Persist what the sources told this client, and report what still blocks work (§69).
///
/// One `source_access` row per blocked `(kind, host)`: the store keys a derived row by its id, so a
/// repeated run refreshes the row it already has instead of minting a second one. A failed write is
/// counted and warned about rather than dropped — the walk still hands back its report, and the next
/// run re-derives the rows it owes.
pub(super) async fn observed(fetcher: &Fetcher, store: &Store) -> Observed {
    let conditions = fetcher.access_conditions().await;
    let blocked_hosts = fetcher
        .blocked_hosts(census_crawl::net::now_iso8601().as_str())
        .await;
    let mut failures = 0_u64;
    if let Err(error) = store.replace_many(Table::SourceAccess, &conditions) {
        failures = failures.saturating_add(1);
        warn!(%error, "source access conditions were not recorded");
    }
    if !blocked_hosts.is_empty() {
        warn!(
            hosts = blocked_hosts.len(),
            "walk stopped on source access blocks; the rosters they cover stay owed"
        );
    }
    Observed {
        conditions,
        blocked_hosts,
        failures,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn http(status: u16) -> CrawlError {
        CrawlError::Fetch(FetchError::Http {
            status,
            url: "https://al.milesplit.com/teams".to_string(),
        })
    }

    /// The predicate that decides whether a walk stops: 403 and 429 end it, and nothing else does.
    ///
    /// Every other status has to keep going, because stopping a 40,000-request walk on a transient
    /// 500 would trade one blocked run for months of uncollected work.
    #[test]
    fn only_refusal_and_rate_limiting_end_a_walk() {
        assert!(refused(&http(403)));
        assert!(refused(&http(429)));
        assert!(refused(&CrawlError::Fetch(FetchError::RateLimited {
            url: "https://al.milesplit.com/teams".to_string(),
            retry_after_secs: Some(60),
        })));
        assert!(!refused(&http(404)));
        assert!(!refused(&http(500)));
        assert!(!refused(&http(200)));
        assert!(!refused(&CrawlError::Fetch(FetchError::Robots(
            "https://al.milesplit.com/teams".to_string()
        ))));
        assert!(!refused(&CrawlError::Fetch(FetchError::TooLarge {
            url: "https://al.milesplit.com/teams".to_string()
        })));
        assert!(!refused(&CrawlError::Schema {
            url: "https://al.milesplit.com/teams".to_string(),
            detail: "unexpected table".to_string(),
        }));
    }
}
