use super::{MAX_QUERY_BYTES, MAX_START};
use crate::domain::identity::ProfileUrl;
use crate::runtime::protocol::SourceResource;
use anyhow::{bail, Result};
use athleticnet_browser::request::{
    endpoint, rankings_spec, safe, safe_text, RankingsAction, RequestAction, RequestSpec,
    SearchBody,
};
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
                // The pipeline's own bio receipts were taken at `level=0`; the census reads this same
                // endpoint at `level=4` (its `HIGH_SCHOOL_LEVEL`) and verified its bio parser and
                // meet-level assertions against those responses. The two are not harmonized: each
                // caller keeps the level its evidence was taken at, because unifying them would move
                // the census onto a response shape no census capture covers.
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
