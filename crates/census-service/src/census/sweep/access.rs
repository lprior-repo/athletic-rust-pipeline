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

/// The run of consecutive page refusals that reads as a host-level wall rather than as one
/// unpublished page.
///
/// One 403 is a page the source will not serve: a closed school answers 403 for its own roster
/// while its neighbours answer 200. Stopping a whole state on it strands every roster behind it,
/// which §62 forbids — one bad source object must not end the census — and leaves the state owing
/// rosters it can never collect. A wall is contiguous, so a short run is enough to tell the two
/// apart: five requests is what it costs.
pub(super) const FORBIDDEN_RUN: u32 = 5;

/// What one failed fetch says about who refused it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Refusal {
    /// The host is limiting this client: the walk stops and its rosters stay owed (§69).
    Host,
    /// One page the source will not serve: the walk continues, and the page is named among the
    /// run's errors instead of being read as a roster with no athletes (§9).
    Page,
    /// Neither, including the outcome of a roster the host did serve.
    None,
}

/// Classify one roster outcome (§69).
///
/// 429, and a rate limit the fetch layer classified before a status was attached, are the host's
/// own words about this client, so they stop the walk at once. 403 is per-URL until a run of them
/// says otherwise: [`RefusalRun`] holds that judgement. The status — not the variant — is what
/// either reads, because the retry loop reports the status it kept seeing once its attempts are
/// spent.
pub(super) fn refusal(error: &CrawlError) -> Refusal {
    match error {
        CrawlError::Fetch(FetchError::Http { status, .. }) if *status == 429 => Refusal::Host,
        CrawlError::Fetch(FetchError::Http { status, .. }) if *status == 403 => Refusal::Page,
        CrawlError::Fetch(FetchError::RateLimited { .. }) => Refusal::Host,
        _ => Refusal::None,
    }
}

/// The consecutive page refusals seen so far, and whether they have become a wall.
#[derive(Debug, Default)]
pub(super) struct RefusalRun {
    consecutive: u32,
}

impl RefusalRun {
    /// Fold one outcome in, answering whether the walk must stop.
    ///
    /// Anything that is not a page refusal breaks the run, because a wall is contiguous: one
    /// roster the host serves is the proof that it is still serving this client.
    pub(super) fn observe(&mut self, refusal: Refusal) -> bool {
        match refusal {
            Refusal::Host => true,
            Refusal::Page => {
                self.consecutive = self.consecutive.saturating_add(1);
                self.consecutive >= FORBIDDEN_RUN
            }
            Refusal::None => {
                self.consecutive = 0;
                false
            }
        }
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

    fn rate_limited() -> CrawlError {
        CrawlError::Fetch(FetchError::RateLimited {
            url: "https://al.milesplit.com/teams".to_string(),
            retry_after_secs: Some(60),
        })
    }

    /// The host's own words about this client end the walk at once.
    #[test]
    fn a_rate_limit_ends_a_walk_at_once() {
        let mut run = RefusalRun::default();
        assert_eq!(refusal(&http(429)), Refusal::Host);
        assert_eq!(refusal(&rate_limited()), Refusal::Host);
        assert!(run.observe(refusal(&http(429))));
    }

    /// One page the source will not serve is not the host refusing the client. Measured
    /// 2026-09-24: Alabama's walk took a single `403 …/teams/698-jo-johnson-closed` and dropped the
    /// remaining 230 rosters unfetched, leaving the state owing rosters no later run could collect.
    #[test]
    fn one_forbidden_page_does_not_end_a_walk() {
        let mut run = RefusalRun::default();
        assert_eq!(refusal(&http(403)), Refusal::Page);
        assert!(!run.observe(Refusal::Page));
    }

    /// A wall is contiguous, and a run of it is what one dead page never produces.
    #[test]
    fn a_contiguous_run_of_forbidden_pages_ends_a_walk() {
        let mut run = RefusalRun::default();
        for _ in 1..FORBIDDEN_RUN {
            assert!(!run.observe(Refusal::Page));
        }
        assert!(run.observe(Refusal::Page));
    }

    /// One roster the host serves is proof it is still serving this client.
    #[test]
    fn a_served_roster_breaks_the_run() {
        let mut run = RefusalRun::default();
        for _ in 1..FORBIDDEN_RUN {
            run.observe(Refusal::Page);
        }
        assert!(!run.observe(Refusal::None));
        assert!(!run.observe(Refusal::Page));
    }

    /// Everything else keeps the walk going: stopping a long walk on a transient 500 would trade
    /// one blocked run for months of uncollected work.
    #[test]
    fn no_other_failure_is_a_refusal() {
        for error in [
            http(404),
            http(500),
            http(200),
            CrawlError::Fetch(FetchError::Robots(
                "https://al.milesplit.com/teams".to_string(),
            )),
            CrawlError::Fetch(FetchError::TooLarge {
                url: "https://al.milesplit.com/teams".to_string(),
            }),
            CrawlError::Schema {
                url: "https://al.milesplit.com/teams".to_string(),
                detail: "unexpected table".to_string(),
            },
        ] {
            assert_eq!(refusal(&error), Refusal::None, "{error}");
        }
    }
}
