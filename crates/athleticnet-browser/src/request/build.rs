use super::{RankingsAction, RequestAction, RequestSpec};
use anyhow::{bail, Result};
use url::Url;

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
    let mut semantic_url = endpoint(origin, &path)?;
    semantic_url
        .query_pairs_mut()
        .append_pair("page", &action.page.to_string());
    if let Some(grade) = action.grade {
        semantic_url
            .query_pairs_mut()
            .append_pair("grades", &grade.to_string());
    }
    let url = endpoint(origin, "/api/v1/tfRankings/GetRankings")?;
    checked(url, semantic_url, RequestAction::Rankings(action))
}

pub fn same_origin(url: &Url, origin: &Url) -> bool {
    url.username().is_empty()
        && url.password().is_none()
        && url.scheme() == origin.scheme()
        && url.host() == origin.host()
        && url.port() == origin.port()
}

/// Whether `url` stays inside the trust boundary of the lane's bound
/// origin: the same scheme and port, no userinfo, and either the same host
/// or a sibling subdomain under the same parent domain
/// (`al.milesplit.com` vs `www.milesplit.com` share `milesplit.com`).
/// The parent must itself contain a dot, so a bare registrable suffix
/// never passes. Anything else fails closed.
pub fn same_site(url: &Url, origin: &Url) -> bool {
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    if url.scheme() != origin.scheme() {
        return false;
    }
    if url.port_or_known_default() != origin.port_or_known_default() {
        return false;
    }
    let (Some(target_host), Some(origin_host)) = (url.host_str(), origin.host_str()) else {
        return false;
    };
    if target_host.eq_ignore_ascii_case(origin_host) {
        return true;
    }
    match (
        strip_first_label(target_host),
        strip_first_label(origin_host),
    ) {
        (Some(target_parent), Some(origin_parent)) => {
            target_parent.eq_ignore_ascii_case(origin_parent) && target_parent.contains('.')
        }
        _ => false,
    }
}

/// Strip the leftmost DNS label (`www.milesplit.com` becomes
/// `milesplit.com`). Returns `None` when there is no parent to name.
fn strip_first_label(host: &str) -> Option<&str> {
    let rest = host.split_once('.')?.1;
    (!rest.is_empty()).then_some(rest)
}

pub fn endpoint(origin: &Url, path: &str) -> Result<Url> {
    if !path.starts_with('/') || path.contains("..") || path.contains(['?', '#', '\\']) {
        bail!("source endpoint path is unsafe");
    }
    let url = origin.join(path)?;
    if !same_origin(&url, origin) {
        bail!("source endpoint escaped configured origin");
    }
    Ok(url)
}

pub fn safe(url: Url, action: RequestAction) -> Result<RequestSpec> {
    let semantic_url = url.clone();
    checked(url, semantic_url, action)
}

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
        headers: Vec::new(),
    })
}

pub fn safe_text(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}
