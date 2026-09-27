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
    })
}

pub fn safe_text(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}
