use super::{RankingsAction, RequestAction, RequestSpec, SearchBody, MAX_QUERY_BYTES, MAX_START};
use crate::domain::identity::ProfileUrl;
use crate::runtime::protocol::SourceResource;
use anyhow::{bail, Result};
use url::Url;

pub(crate) fn build(origin: &Url, resource: &SourceResource) -> Result<RequestSpec> {
    match resource {
        SourceResource::Search {
            query,
            sport,
            start,
        } => search(origin, query, *sport, *start),
        SourceResource::Bio { athlete_id, sport } => {
            let mut url = endpoint(origin, "/api/v1/AthleteBio/GetAthleteBioData")?;
            url.query_pairs_mut()
                .append_pair("athleteId", &athlete_id.get().to_string())
                .append_pair("sport", sport.api_code())
                .append_pair("level", "0");
            safe(url, RequestAction::Fetch { body: None })
        }
        SourceResource::ProfileHtml { profile_url } => profile(origin, profile_url),
        SourceResource::Team {
            team_id,
            sport,
            season,
        } => team(origin, *team_id, *sport, *season),
        SourceResource::Rankings {
            list_id,
            gender,
            grade,
            event_short,
            page,
            capture,
            ..
        } => rankings_spec(
            origin,
            RankingsAction {
                list_id: *list_id,
                gender: gender.to_owned(),
                grade: *grade,
                event_short: event_short.to_owned(),
                page: *page,
                capture: capture.clone(),
            },
        ),
    }
}

fn search(
    origin: &Url,
    query: &str,
    sport: crate::domain::evidence::Sport,
    start: u32,
) -> Result<RequestSpec> {
    if start > MAX_START || !safe_text(query, MAX_QUERY_BYTES) {
        bail!("search input is outside bounded safe syntax");
    }
    let body = SearchBody {
        q: query.to_owned(),
        fq: format!("t:a a:{}", sport.api_code()),
        start,
    };
    safe(
        endpoint(origin, "/Search.aspx/runSearch")?,
        RequestAction::Fetch { body: Some(body) },
    )
}

fn profile(origin: &Url, profile_url: &ProfileUrl) -> Result<RequestSpec> {
    let parsed = Url::parse(profile_url.as_str())?;
    let path = parsed.path();
    if path.is_empty() || path.contains("..") || path.contains(['?', '#', '\\']) {
        bail!("profile path is unsafe");
    }
    safe(endpoint(origin, path)?, RequestAction::Fetch { body: None })
}

fn team(
    origin: &Url,
    team_id: u64,
    sport: crate::domain::evidence::Sport,
    season: u16,
) -> Result<RequestSpec> {
    if team_id == 0 || season == 0 {
        bail!("team identifier and season must be nonzero");
    }
    let mut url = endpoint(origin, "/api/v1/TeamNav/Team")?;
    url.query_pairs_mut()
        .append_pair("team", &team_id.to_string())
        .append_pair("sport", sport.api_code())
        .append_pair("season", &season.to_string());
    safe(url, RequestAction::Fetch { body: None })
}

/// Build the request for one rankings page. The physical `url` is the site's own
/// rankings API endpoint while `semantic_url` stays the legacy listing URL, so
/// cache keys, checkpoints, and receipts keep their established identity.
pub(crate) fn rankings_spec(origin: &Url, action: RankingsAction) -> Result<RequestSpec> {
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
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        bail!("source URL contains forbidden authority data");
    }
    Ok(RequestSpec {
        semantic_url: semantic_url.to_string(),
        url,
        action: RequestAction::Rankings(action),
    })
}

fn endpoint(origin: &Url, path: &str) -> Result<Url> {
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

fn safe(url: Url, action: RequestAction) -> Result<RequestSpec> {
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        bail!("source URL contains forbidden authority data");
    }
    Ok(RequestSpec {
        semantic_url: url.to_string(),
        url,
        action,
    })
}

fn safe_text(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}
