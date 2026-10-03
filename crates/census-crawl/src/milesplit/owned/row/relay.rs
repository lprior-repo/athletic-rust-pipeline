use super::{provider_id, EventKind, Gender, OwnedPerformance, OwnedRejectionKind, Scalar};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireTeamRelay<'a> {
    #[serde(borrow)]
    id: Scalar<'a>,
    #[serde(borrow)]
    meet_id: Scalar<'a>,
    #[serde(borrow)]
    meet_results_id: Scalar<'a>,
    #[serde(borrow)]
    team_id: Scalar<'a>,
    team_name: &'a str,
    gender: &'a str,
    mark: &'a str,
}

pub(super) fn reject(
    value: &Value,
    meet_id: u64,
    kind: &EventKind,
) -> Result<OwnedPerformance, (OwnedRejectionKind, String)> {
    let row: WireTeamRelay<'_> = WireTeamRelay::deserialize(value)
        .map_err(|error| (OwnedRejectionKind::MalformedRow, error.to_string()))?;
    let result_id = provider_id(&row.id)?;
    let published_meet = provider_id(&row.meet_id)?;
    let result_set_id = provider_id(&row.meet_results_id)?;
    let team_id = provider_id(&row.team_id)?;
    if published_meet != meet_id {
        return Err((
            OwnedRejectionKind::ForeignMeet,
            format!("expected meet {meet_id}, found {published_meet}"),
        ));
    }
    super::super::context::validate_context(value)?;
    if Gender::parse_milesplit(row.gender) == Gender::Unknown || row.team_name.trim().is_empty() {
        return Err((
            OwnedRejectionKind::InvalidContext,
            "team relay requires known gender and nonempty published teamName".to_string(),
        ));
    }
    if !super::no_mark(row.mark) && crate::milesplit::mark::parse_published_time(row.mark).is_none()
    {
        return Err((
            OwnedRejectionKind::InvalidContext,
            "malformed published mark".to_string(),
        ));
    }
    Err((OwnedRejectionKind::TeamRelay, format!(
        "published whole-team relay result {result_id}, meet {published_meet}, result set {result_set_id}, team {team_id} ({}) event {kind:?}, mark {}; no individual performance, membership or leg split is attributed; source-total completeness is not established",
        row.team_name, row.mark,
    )))
}
