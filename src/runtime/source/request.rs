use crate::domain::identity::ProfileUrl;
use crate::runtime::protocol::RankingsCapture;
use crate::runtime::protocol::SourceResource;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub(crate) enum RequestAction {
    Fetch { body: Option<SearchBody> },
    Rankings(RankingsAction),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct RankingsAction {
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
    pub(crate) action: RequestAction,
}

impl RequestSpec {
    pub(crate) fn body(&self) -> Option<&SearchBody> {
        match &self.action {
            RequestAction::Fetch { body } => body.as_ref(),
            RequestAction::Rankings(_) => None,
        }
    }
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
        } => rankings(
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

fn rankings(origin: &Url, action: RankingsAction) -> Result<RequestSpec> {
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
    let mut url = endpoint(origin, &path)?;
    url.query_pairs_mut()
        .append_pair("page", &action.page.to_string());
    if let Some(grade) = action.grade {
        url.query_pairs_mut()
            .append_pair("grades", &grade.to_string());
    }
    safe(url, RequestAction::Rankings(action))
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
            serde_json::to_value(request.body()).expect("json"),
            serde_json::json!({"q":"Ada Example","fq":"t:a a:tf","start":12})
        );
    }

    #[test]
    fn captured_fetch_requests_survive_durable_serialization() -> anyhow::Result<()> {
        let origin = Url::parse("http://127.0.0.1:8080/")?;
        let resources = [
            SourceResource::Search {
                query: "Ada Example".to_owned(),
                sport: Sport::TrackField,
                start: 12,
            },
            SourceResource::Bio {
                athlete_id: AthleteId::new(7)?,
                sport: Sport::CrossCountry,
            },
        ];
        for resource in resources {
            let request = build(&origin, &resource)?;
            let bytes = serde_json::to_vec(&request)?;
            let restored: RequestSpec = serde_json::from_slice(&bytes)?;
            assert_eq!(restored, request);
        }
        Ok(())
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
