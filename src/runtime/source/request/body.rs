use super::SearchBody;
use serde::Serialize;

/// Helper type for serializing qParams grades as a JSON array.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RankingsQParamsInner<'a> {
    pub grades: &'a [u8],
    pub page: u32,
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
