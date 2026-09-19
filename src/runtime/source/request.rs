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

/// Helper type for serializing qParams grades as a JSON array.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RankingsQParamsInner<'a> {
    pub grades: &'a [u8],
    pub page: u32,
}

/// Convert an optional grade into RankingsQParamsInner, borrowing from the action.
fn rankings_q_params(grade: &Option<u8>, page: u32) -> RankingsQParamsInner<'_> {
    let grades = match grade {
        Some(g) => std::slice::from_ref(g),
        None => &[],
    };
    RankingsQParamsInner { grades, page }
}
/// Wire-body struct for Rankings POST body.  Borrows from RankingsAction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RankingsQuery<'a> {
    pub report_type: &'static str,
    pub mode: &'static str,
    pub div_list_id: u64,
    pub indoor: Option<()>,
    pub event_short: &'a str,
    pub gender: &'a str,
    pub q_params: RankingsQParamsInner<'a>,
    pub qualifying_list_key: &'static str,
    pub version: u8,
    pub debug: &'static str,
}

/// Borrowed wire-body enum returned by RequestSpec::body().
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub(crate) enum RequestBody<'a> {
    Search(&'a SearchBody),
    Rankings(RankingsQuery<'a>),
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
    pub(crate) fn body(&self) -> Option<RequestBody<'_>> {
        match &self.action {
            RequestAction::Fetch { body } => body.as_ref().map(RequestBody::Search),
            RequestAction::Rankings(action) => Some(RequestBody::Rankings(RankingsQuery {
                report_type: "div",
                mode: "list",
                div_list_id: action.list_id,
                indoor: None,
                event_short: &action.event_short,
                gender: &action.gender,
                q_params: rankings_q_params(&action.grade, action.page),
                qualifying_list_key: "",
                version: 2,
                debug: "",
            })),
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
    // -- Rankings body encoding and wire layout --

    #[test]
    fn rankings_query_encodes_exact_measured_body() {
        let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
        let request = rankings_spec(
            &origin,
            RankingsAction {
                list_id: 168416,
                gender: "m".into(),
                grade: Some(11),
                event_short: "100m".into(),
                page: 1,
                capture: RankingsCapture::Navigation,
            },
        )
        .expect("request");
        // Verify body serialises to the byte-verified measured request body.
        let body = request.body().expect("has body");
        match body {
            RequestBody::Rankings(_) => {}
            _ => panic!("expected Rankings body"),
        }
        let json = serde_json::to_string(&body).expect("json");
        let expected = r#"{"reportType":"div","mode":"list","divListId":168416,"indoor":null,"eventShort":"100m","gender":"m","qParams":{"grades":[11],"page":1},"qualifyingListKey":"","version":2,"debug":""}"#;
        assert_eq!(json, expected);
    }

    #[test]
    fn rankings_grade_none_emits_empty_array_not_null() {
        let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
        let request = rankings_spec(
            &origin,
            RankingsAction {
                list_id: 168416,
                gender: "f".into(),
                grade: None,
                event_short: "200m".into(),
                page: 2,
                capture: RankingsCapture::Results,
            },
        )
        .expect("request");
        let json = serde_json::to_string(&request.body().expect("body")).expect("json");
        assert!(json.contains(r#""grades":[]"#));
        assert!(!json.contains(r#""grades":null"#));
    }

    #[test]
    fn rankings_semantic_url_matches_legacy_ui_url() {
        let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
        // With grade
        let req = rankings_spec(
            &origin,
            RankingsAction {
                list_id: 168416,
                gender: "m".into(),
                grade: Some(11),
                event_short: "100m".into(),
                page: 3,
                capture: RankingsCapture::Navigation,
            },
        )
        .expect("request");
        assert_eq!(
            req.semantic_url,
            "http://127.0.0.1:8080/TrackAndField/rankings/list/168416/m/100m/?page=3&grades=11"
        );
        // Without grade — use a valid event_short (empty fails safe_text validation)
        let req2 = rankings_spec(
            &origin,
            RankingsAction {
                list_id: 99,
                gender: "f".into(),
                grade: None,
                event_short: "TF".into(),
                page: 1,
                capture: RankingsCapture::Results,
            },
        )
        .expect("request");
        assert_eq!(
            req2.semantic_url,
            "http://127.0.0.1:8080/TrackAndField/rankings/list/99/f/TF/?page=1"
        );
    }

    #[test]
    fn rankings_physical_url_is_api_endpoint_empty_query() {
        let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
        let request = rankings_spec(
            &origin,
            RankingsAction {
                list_id: 168416,
                gender: "m".into(),
                grade: Some(11),
                event_short: "100m".into(),
                page: 1,
                capture: RankingsCapture::Navigation,
            },
        )
        .expect("request");
        assert_eq!(
            request.url.as_str(),
            "http://127.0.0.1:8080/api/v1/tfRankings/GetRankings"
        );
        assert!(request.url.query().is_none());
    }

    #[test]
    fn rankings_rejects_zero_page_and_oversized_event() {
        let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
        assert!(rankings_spec(
            &origin,
            RankingsAction {
                list_id: 1,
                gender: "m".into(),
                grade: None,
                event_short: "100m".into(),
                page: 0,
                capture: RankingsCapture::Navigation,
            },
        )
        .is_err());
        // Event text exceeding 64 bytes
        let long_event = "x".repeat(65);
        assert!(rankings_spec(
            &origin,
            RankingsAction {
                list_id: 1,
                gender: "m".into(),
                grade: None,
                event_short: long_event,
                page: 1,
                capture: RankingsCapture::Navigation,
            },
        )
        .is_err());
    }
}
