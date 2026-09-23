//! Spec construction the transport performs for itself, plus the URL bounding both producers use.
//!
//! `rankings_spec` is not only a producer concern: the rankings transport rebuilds the spec for
//! each page it captures, so it lives here. The pipeline crate keeps the `SourceResource` mapping
//! (`build`, `search`, `profile`, `team`) and imports these helpers, because a spec that escapes
//! the configured origin is rejected the same way whoever asks for it.

use super::{RankingsAction, RequestAction, RequestSpec};
use anyhow::{bail, Result};
use url::Url;

/// Build the request for one rankings page. The physical `url` is the site's own
/// rankings API endpoint while `semantic_url` stays the legacy listing URL, so
/// cache keys, checkpoints, and receipts keep their established identity.
pub fn rankings_spec(origin: &Url, action: RankingsAction) -> Result<RequestSpec> {
    if action.list_id == 0 {
        bail!("rankings list_id must be nonzero");
    }
    if !safe_text(&action.event_short, 64) || action.page == 0 {
        bail!("rankings event and page must be bounded");
    }
    let path = format!(
        "/TrackAndField/rankings/list/{}/{}/{}/",
        action.list_id, action.gender, action.event_short
    );
    // Build semantic_url: legacy UI path + query params
    let mut semantic_url = endpoint(origin, &path)?;
    semantic_url
        .query_pairs_mut()
        .append_pair("page", &action.page.to_string());
    if let Some(grade) = action.grade {
        semantic_url
            .query_pairs_mut()
            .append_pair("grades", &grade.to_string());
    }
    // Physical url: API endpoint, no query string
    let url = endpoint(origin, "/api/v1/tfRankings/GetRankings")?;
    checked(url, semantic_url, RequestAction::Rankings(action))
}

/// Join a bounded path onto the configured origin, refusing anything that escapes it.
pub fn endpoint(origin: &Url, path: &str) -> Result<Url> {
    if !path.starts_with('/') || path.contains("..") || path.contains(['?', '#', '\\']) {
        bail!("source endpoint path is unsafe");
    }
    let url = origin.join(path)?;
    if url.scheme() != origin.scheme() || url.host() != origin.host() || url.port() != origin.port()
    {
        bail!("source endpoint escaped configured origin");
    }
    Ok(url)
}

/// Wrap a URL and its action in a spec, refusing authority data a receipt must never carry.
///
/// The URL is both the physical address and the identity a receipt cites. A producer whose two
/// differ - rankings - builds the pair through `checked` instead.
pub fn safe(url: Url, action: RequestAction) -> Result<RequestSpec> {
    let semantic_url = url.clone();
    checked(url, semantic_url, action)
}

/// Build one spec from a physical URL and the distinct identity a receipt cites for it.
///
/// Whichever of the two the receipt ends up citing has to be an address without authority data, so
/// both are refused when they carry credentials or a fragment.
fn checked(url: Url, semantic_url: Url, action: RequestAction) -> Result<RequestSpec> {
    for candidate in [&url, &semantic_url] {
        if !candidate.username().is_empty()
            || candidate.password().is_some()
            || candidate.fragment().is_some()
        {
            bail!("source URL contains forbidden authority data");
        }
    }
    Ok(RequestSpec {
        url,
        semantic_url: semantic_url.to_string(),
        action,
    })
}

/// Bounded, control-character-free input check for any value that reaches a spec.
pub fn safe_text(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}
