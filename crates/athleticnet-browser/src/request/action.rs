use super::{RankingsQParamsInner, RankingsQuery, RequestBody};
use crate::protocol::RankingsCapture;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum RequestAction {
    Fetch { body: Option<SearchBody> },
    Rankings(RankingsAction),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RankingsAction {
    pub list_id: u64,
    pub gender: String,
    pub grade: Option<u8>,
    pub event_short: String,
    pub page: u32,
    pub capture: RankingsCapture,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestSpec {
    pub url: Url,
    pub semantic_url: String,
    pub action: RequestAction,
}

impl RequestSpec {
    pub fn body(&self) -> Option<RequestBody<'_>> {
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

/// Convert an optional grade into RankingsQParamsInner, borrowing from the action.
fn rankings_q_params(grade: &Option<u8>, page: u32) -> RankingsQParamsInner<'_> {
    let grades = match grade {
        Some(g) => std::slice::from_ref(g),
        None => &[],
    };
    RankingsQParamsInner { grades, page }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchBody {
    pub q: String,
    pub fq: String,
    pub start: u32,
}
