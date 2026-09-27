use super::SearchBody;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RankingsQParamsInner<'a> {
    pub grades: &'a [u8],
    pub page: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RankingsQuery<'a> {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum RequestBody<'a> {
    Search(&'a SearchBody),
    Rankings(RankingsQuery<'a>),
}
