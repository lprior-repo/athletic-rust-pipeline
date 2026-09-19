use crate::domain::identity::ProfileUrl;
use crate::runtime::protocol::SourceResource;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use url::Url;
use crate::runtime::protocol::RankingsCapture;
use crate::domain::identity::EvidenceDigest;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub(crate) enum RequestAction {
    Fetch,
    Rankings(RankingsAction),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct RankingsAction {
    pub collection: EvidenceDigest,
    pub list_id: u64,
    pub gender: String,
    pub grade: Option<u8>,
    pub event_short: String,
    pub page: u32,
    pub capture: RankingsCapture,
}
pub(crate) const MAX_START: u32 = 1_000_000;
pub(crate) const MAX_QUERY_BYTES: usize = 2_048;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RequestSpec {
    pub(crate) url: Url,
    pub(crate) semantic_url: String,
    pub(crate) body: Option<SearchBody>,
    pub(crate) action: RequestAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchBody {
    pub q: String,
    pub fq: String,
    pub start: u32,
}

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
            safe(url, None, RequestAction::Fetch)
        }
        SourceResource::ProfileHtml { profile_url } => profile(origin, profile_url),
        SourceResource::Team {
            team_id,
            sport,
            season,
        } => team(origin, *team_id, *sport, *season),
        SourceResource::Rankings {
            collection,
            list_id,
            gender,
            grade,
            event_short,
            page,
            capture,
        } => rankings(
            origin,
            collection.clone(),
            *list_id,
            gender,
            *grade,
            event_short,
            *page,
            capture,
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
    safe(endpoint(origin, "/Search.aspx/runSearch")?, Some(body), RequestAction::Fetch)
}

fn profile(origin: &Url, profile_url: &ProfileUrl) -> Result<RequestSpec> {
    let parsed = Url::parse(profile_url.as_str())?;
    let path = parsed.path();
    if path.is_empty() || path.contains("..") || path.contains(['?', '#', '\\']) {
        bail!("profile path is unsafe");
    }
    safe(endpoint(origin, path)?, None, RequestAction::Fetch)
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
    safe(url, None, RequestAction::Fetch)
}

fn rankings(
    origin: &Url,
    collection: EvidenceDigest,
    list_id: u64,
    gender: &str,
    grade: Option<u8>,
    event_short: &str,
    page: u32,
    capture: &RankingsCapture,
) -> Result<RequestSpec> {
    if list_id == 0 {
        bail!("rankings list_id must be nonzero");
    }
    if !safe_text(event_short, 64) || page == 0 {
        bail!("rankings event and page must be bounded");
    }
    let path = format!("/TrackAndField/rankings/list/{list_id}/{gender}/{event_short}/");
    let mut url = endpoint(origin, &path)?;
    url.query_pairs_mut().append_pair("page", &page.to_string());
    if let Some(grade) = grade {
        url.query_pairs_mut().append_pair("grades", &grade.to_string());
    }
    let action = RequestAction::Rankings(RankingsAction {
        collection: collection.clone(),
        list_id,
        gender: gender.to_owned(),
        grade,
        event_short: event_short.to_owned(),
        page,
        capture: capture.clone(),
    });
    safe(url, None, action)
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

fn safe(url: Url, body: Option<SearchBody>, action: RequestAction) -> Result<RequestSpec> {
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        bail!("source URL contains forbidden authority data");
    }
    Ok(RequestSpec {
        semantic_url: url.to_string(),
        url,
        body,
        action,
    })
}

fn safe_text(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::evidence::Sport;
    use crate::domain::identity::AthleteId;

    #[test]
    fn search_body_is_exact_and_encoded() {
        let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
        let request = build(
            &origin,
            &SourceResource::Search {
                query: "Ada Example".to_owned(),
                sport: Sport::TrackField,
                start: 12,
            },
        )
        .expect("request");
        assert_eq!(request.url.path(), "/Search.aspx/runSearch");
        assert_eq!(
            serde_json::to_value(request.body).expect("json"),
            serde_json::json!({"q":"Ada Example","fq":"t:a a:tf","start":12})
        );
    }

    #[test]
    fn unsafe_search_and_profile_inputs_are_rejected() {
        let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
        assert!(build(
            &origin,
            &SourceResource::Search {
                query: "x\ninvalid".to_owned(),
                sport: Sport::CrossCountry,
                start: 0
            }
        )
        .is_err());
        let id = AthleteId::new(7).expect("id");
        let profile = ProfileUrl::parse("https://athletic.net/athlete/7").expect("profile");
        assert_eq!(profile.athlete_id(), id);
        assert!(build(
            &origin,
            &SourceResource::ProfileHtml {
                profile_url: profile
            }
        )
        .is_ok());
    }
}
