use super::{ASSOCIATION, HOST};
use crate::{CrawlError, CrawlResult};
use census_domain::school_directory::StateRecordId;
use url::Url;

pub(super) fn artifact(detail: impl Into<String>) -> CrawlError {
    CrawlError::Invariant {
        detail: detail.into(),
    }
}

fn validated(raw: &str) -> CrawlResult<(Url, StateRecordId)> {
    let url = Url::parse(raw)
        .or_else(|_| Url::parse(HOST)?.join(raw))
        .map_err(|error| artifact(error.to_string()))?;
    if url.scheme() != "https"
        || url.host_str() != Some("uhsaa.org")
        || url.path() != "/school-directory/"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
    {
        return Err(artifact(
            "UHSAA profile URL is outside the public school directory",
        ));
    }
    let ids: Vec<_> = url
        .query_pairs()
        .filter(|(key, _)| key == "schoolID")
        .collect();
    let id = match ids.as_slice() {
        [(_, id)] if !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()) => id,
        _ => return Err(artifact("UHSAA profile URL has no unique numeric schoolID")),
    };
    let numeric = id
        .parse::<u64>()
        .map_err(|error| artifact(error.to_string()))?;
    if numeric == 0 || numeric.to_string() != id.as_ref() {
        return Err(artifact(
            "UHSAA schoolID is not a canonical positive identity",
        ));
    }
    let id = StateRecordId::parse(id)
        .map_err(|error| artifact(format!("{ASSOCIATION} owner: {error}")))?;
    Ok((url, id))
}

pub(super) fn school_id(raw: &str) -> CrawlResult<StateRecordId> {
    validated(raw).map(|(_, id)| id)
}

pub(super) fn request_url(raw: &str) -> CrawlResult<String> {
    let (parsed, _) = validated(raw)?;
    let encoded: String = parsed.into();
    if encoded.chars().any(char::is_whitespace) {
        return Err(artifact("UHSAA profile request URL retains whitespace"));
    }
    Ok(encoded)
}
