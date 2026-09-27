
use tracing::warn;

use census_crawl::net::{FetchError, Fetcher};
use census_crawl::CrawlError;
use census_domain::model::SourceAccessCondition;
use census_store::{Store, Table};

pub(super) const FORBIDDEN_RUN: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Refusal {
    Host,
    Page,
    None,
}

pub(super) fn refusal(error: &CrawlError) -> Refusal {
    match error {
        CrawlError::Fetch(FetchError::Http { status, .. }) if *status == 429 => Refusal::Host,
        CrawlError::Fetch(FetchError::Http { status, .. }) if *status == 403 => Refusal::Page,
        CrawlError::Fetch(FetchError::RateLimited { .. }) => Refusal::Host,
        _ => Refusal::None,
    }
}

#[derive(Debug, Default)]
pub(super) struct RefusalRun {
    consecutive: u32,
}

impl RefusalRun {
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

pub(super) struct Observed {
    pub(super) conditions: Vec<SourceAccessCondition>,
    pub(super) blocked_hosts: Vec<String>,
    pub(super) failures: u64,
}

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

    #[test]
    fn a_rate_limit_ends_a_walk_at_once() {
        let mut run = RefusalRun::default();
        assert_eq!(refusal(&http(429)), Refusal::Host);
        assert_eq!(refusal(&rate_limited()), Refusal::Host);
        assert!(run.observe(refusal(&http(429))));
    }

    #[test]
    fn one_forbidden_page_does_not_end_a_walk() {
        let mut run = RefusalRun::default();
        assert_eq!(refusal(&http(403)), Refusal::Page);
        assert!(!run.observe(Refusal::Page));
    }

    #[test]
    fn a_contiguous_run_of_forbidden_pages_ends_a_walk() {
        let mut run = RefusalRun::default();
        for _ in 1..FORBIDDEN_RUN {
            assert!(!run.observe(Refusal::Page));
        }
        assert!(run.observe(Refusal::Page));
    }

    #[test]
    fn a_served_roster_breaks_the_run() {
        let mut run = RefusalRun::default();
        for _ in 1..FORBIDDEN_RUN {
            run.observe(Refusal::Page);
        }
        assert!(!run.observe(Refusal::None));
        assert!(!run.observe(Refusal::Page));
    }

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
