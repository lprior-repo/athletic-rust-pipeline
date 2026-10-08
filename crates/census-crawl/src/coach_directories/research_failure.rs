use crate::net::FetchError;
use census_domain::model::ContactResearchOutcome as Outcome;

pub(crate) fn fetch(error: &FetchError) -> Outcome {
    match error {
        FetchError::Http { status: code, .. } => status(*code),
        FetchError::Policy { .. }
        | FetchError::Offline { .. }
        | FetchError::OriginHeld { .. }
        | FetchError::RateLimited { .. }
        | FetchError::BrowserLane {
            retryable: false, ..
        } => Outcome::Blocked,
        FetchError::TooLarge { .. } => Outcome::Partial,
        FetchError::Transport { .. }
        | FetchError::Cache { .. }
        | FetchError::OriginLockIo { .. }
        | FetchError::Timeout { .. }
        | FetchError::BrowserLane {
            retryable: true, ..
        }
        | FetchError::InvalidUrl { .. }
        | FetchError::Decode { .. }
        | FetchError::Encode { .. }
        | FetchError::Client { .. }
        | FetchError::Invariant { .. } => Outcome::Failed,
    }
}

pub(crate) fn status(code: u16) -> Outcome {
    match code {
        401 | 403 | 429 => Outcome::Blocked,
        _ => Outcome::Failed,
    }
}

pub(super) fn crawl(error: &crate::CrawlError) -> Outcome {
    use crate::CrawlError;
    match error {
        CrawlError::Fetch(error) => fetch(error),
        CrawlError::Directory(error) if error.is_resource() => Outcome::Partial,
        CrawlError::Directory(_) => Outcome::Ambiguous,
        CrawlError::Resource { .. } => Outcome::Partial,
        CrawlError::Schema { .. }
        | CrawlError::Domain(_)
        | CrawlError::Invariant { .. }
        | CrawlError::Performance(_)
        | CrawlError::PerformanceDateUnknown { .. } => Outcome::Ambiguous,
        CrawlError::RegexInit { .. }
        | CrawlError::Decode { .. }
        | CrawlError::Encode { .. }
        | CrawlError::Canonical { .. }
        | CrawlError::Store(_)
        | CrawlError::Arithmetic { .. }
        | CrawlError::EventIdentity(_)
        | CrawlError::Specification(_)
        | CrawlError::Io { .. }
        | CrawlError::DirectoryArtifact { .. } => Outcome::Failed,
    }
}
