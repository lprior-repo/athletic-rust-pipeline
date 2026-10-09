use crate::net::FetchError;
use crate::CrawlError;

pub fn sample_rows<T: std::borrow::Borrow<super::parse::DirectorySchool>>(
    rows: &[T],
) -> Vec<&super::parse::DirectorySchool> {
    let step = rows.len().saturating_div(4).max(1);
    rows.iter()
        .step_by(step)
        .take(4)
        .map(std::borrow::Borrow::borrow)
        .collect()
}

pub fn round_half_even(numerator: usize, denominator: usize, scale: usize) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let scaled = numerator.saturating_mul(scale);
    let quotient = scaled.checked_div(denominator).map_or(0, |value| value);
    let remainder = scaled.checked_rem(denominator).map_or(0, |value| value);
    let doubled = remainder.saturating_mul(2);
    let tie = doubled == denominator && quotient.checked_rem(2).map_or(0, |value| value) == 1;
    let adjusted = if doubled > denominator || tie {
        quotient.saturating_add(1)
    } else {
        quotient
    };
    let rounded = i32::try_from(adjusted).map_or(f64::from(i32::MAX), f64::from);
    let divisor = i32::try_from(scale).map_or(f64::from(i32::MAX), f64::from);
    rounded / divisor
}

pub fn classify_error(error: &CrawlError) -> (&'static str, String) {
    let status = match error {
        CrawlError::Fetch(error) => match error {
            FetchError::Http { .. }
            | FetchError::TooLarge { .. }
            | FetchError::BrowserLane { .. } => "http",
            FetchError::RateLimited { .. } => "rate_limited",
            FetchError::Cooldown { .. } => "cooldown",
            FetchError::Transport { .. } | FetchError::Client { .. } => "transport",
            FetchError::Timeout { .. } => "timeout",
            FetchError::Decode { .. } => "json",
            FetchError::Policy { .. } | FetchError::OriginHeld { .. } => "policy",
            FetchError::Offline { .. } => "offline",
            FetchError::Cache { .. }
            | FetchError::InvalidUrl { .. }
            | FetchError::Encode { .. }
            | FetchError::OriginLockIo { .. }
            | FetchError::Invariant { .. } => "invariant",
        },
        CrawlError::Decode { .. } => "json",
        CrawlError::Schema { .. } => "schema",
        CrawlError::Store(_) => "store",
        CrawlError::Io { .. } => "io",
        CrawlError::RegexInit { .. }
        | CrawlError::Encode { .. }
        | CrawlError::Canonical { .. }
        | CrawlError::Domain(_)
        | CrawlError::Arithmetic { .. }
        | CrawlError::Directory(_)
        | CrawlError::Resource { .. }
        | CrawlError::EventIdentity(_)
        | CrawlError::Specification(_)
        | CrawlError::Performance(_)
        | CrawlError::PerformanceDateUnknown { .. }
        | CrawlError::Invariant { .. }
        | CrawlError::DirectoryArtifact { .. } => "invariant",
    };
    (status, error.to_string())
}
