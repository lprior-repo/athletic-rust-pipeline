use tracing::warn;

use census_crawl::net::{FetchError, Fetcher};
use census_crawl::CrawlError;
use census_domain::model::SourceAccessCondition;
use census_store::Store;

mod failure;
pub use failure::SourceAccessFailure;

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
                let Some(consecutive) = self.consecutive.checked_add(1) else {
                    return true;
                };
                self.consecutive = consecutive;
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
    pub(super) evidence_losses: Vec<census_crawl::net::RefusalEvidenceLoss>,
    pub(super) failures: u64,
}

pub(super) async fn observed(
    fetcher: &Fetcher,
    store: &Store,
) -> census_crawl::CrawlResult<Observed> {
    let conditions = fetcher.access_conditions().await?;
    let blocked_hosts = fetcher
        .blocked_hosts(census_crawl::net::now_iso8601().as_str())
        .await;
    let evidence_losses = fetcher.refusal_evidence_losses().await;
    let mut failures = 0_u64;
    if let Err(error) = persist(store, &conditions) {
        failures = 1;
        warn!(%error, "source access conditions were not recorded");
    }
    if !blocked_hosts.is_empty() {
        warn!(
            hosts = blocked_hosts.len(),
            "walk stopped on source access blocks; the rosters they cover stay owed"
        );
    }
    if !evidence_losses.is_empty() {
        warn!(
            losses = evidence_losses.len(),
            "refusal captures were not retained; their refusals keep their classification without an archived body"
        );
    }
    Ok(Observed {
        conditions,
        blocked_hosts,
        evidence_losses,
        failures,
    })
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
            CrawlError::Fetch(FetchError::Policy {
                detail: "the request exceeds what the registry admits for this host".to_string(),
            }),
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

pub fn combined_failure(
    primary: &impl std::fmt::Display,
    secondary: &impl std::fmt::Display,
) -> CrawlError {
    use std::fmt::Write;
    struct Bounded(String);
    impl Write for Bounded {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            let room = 4096usize.saturating_sub(self.0.len());
            let mut end = value.len().min(room);
            while !value.is_char_boundary(end) {
                end = end.saturating_sub(1);
            }
            self.0.push_str(value.get(..end).ok_or(std::fmt::Error)?);
            Ok(())
        }
    }
    let render = |value: &dyn std::fmt::Display| {
        let mut out = Bounded(String::new());
        out.0
            .try_reserve_exact(4096)
            .map_err(|_| CrawlError::Resource {
                resource: "source access error rendering",
                requested: 4096,
                limit: 4096,
            })?;
        write!(&mut out, "{value}").map_err(|_| CrawlError::Invariant {
            detail: "source access error rendering failed".to_string(),
        })?;
        Ok::<_, CrawlError>(out.0)
    };
    match (render(primary), render(secondary)) {
        (Ok(primary), Ok(secondary)) => CrawlError::Invariant {
            detail: format!("{primary}; retaining access evidence also failed: {secondary}"),
        },
        (Err(error), _) | (_, Err(error)) => error,
    }
}

pub(crate) fn stage(
    batch: &mut census_store::StoreBatch<'_>,
    conditions: &[SourceAccessCondition],
) -> census_store::StoreResult<()> {
    batch.merge_source_access(conditions)
}

fn persist(store: &Store, conditions: &[SourceAccessCondition]) -> census_store::StoreResult<()> {
    let mut batch = store.write_batch();
    stage(&mut batch, conditions)?;
    batch.commit()
}
