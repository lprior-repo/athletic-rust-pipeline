use super::{OwnedCohort, OwnedPerformance, OwnedRejectionKind};
use census_domain::model::{
    person_key, EventKind, Gender, GradYear, Mark, SourceIdentity, SourceNamespace,
};
use serde::Deserialize;
use serde_json::Value;

mod relay;

#[derive(Deserialize)]
#[serde(untagged)]
enum Scalar<'a> {
    Text(&'a str),
    Number(u64),
}

impl Scalar<'_> {
    fn id(&self) -> Option<u64> {
        match self {
            Self::Number(value) => (*value > 0).then_some(*value),
            Self::Text(value) => value.parse::<u64>().ok().filter(|id| {
                *id > 0
                    && !value.starts_with('0')
                    && value.bytes().all(|byte| byte.is_ascii_digit())
            }),
        }
    }

    fn text(&self) -> String {
        match self {
            Self::Text(value) => (*value).to_string(),
            Self::Number(value) => value.to_string(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WirePerformance<'a> {
    #[serde(borrow)]
    id: Scalar<'a>,
    #[serde(borrow)]
    meet_id: Scalar<'a>,
    #[serde(borrow)]
    meet_results_id: Scalar<'a>,
    #[serde(borrow)]
    athlete_id: Option<Scalar<'a>>,
    #[serde(borrow)]
    team_id: Scalar<'a>,
    first_name: &'a str,
    last_name: &'a str,
    gender: &'a str,
    #[serde(borrow)]
    grad_year: Option<Scalar<'a>>,
    mark: &'a str,
    profile_url: Option<&'a str>,
    status_code: Option<&'a str>,
}

pub(super) fn parse_row(
    value: Value,
    meet_id: u64,
    locator: String,
) -> Result<OwnedPerformance, (OwnedRejectionKind, String)> {
    let event_kind = value
        .get("eventName")
        .and_then(Value::as_str)
        .map(crate::hytek::hytek_event_kind);
    if let Some(kind) = event_kind.as_ref().filter(|kind| kind.is_relay()) {
        return relay::reject(&value, meet_id, kind);
    }
    let row: WirePerformance<'_> = WirePerformance::deserialize(&value)
        .map_err(|error| (OwnedRejectionKind::MalformedRow, error.to_string()))?;
    let event_kind = event_kind.ok_or_else(|| {
        (
            OwnedRejectionKind::MalformedRow,
            "missing or nontext published eventName".to_string(),
        )
    })?;
    let source_athlete = owner(&row)?;
    super::context::validate_context(&value)?;
    validate_name(&row)?;
    let published_meet = provider_id(&row.meet_id)?;
    if published_meet != meet_id {
        return Err((
            OwnedRejectionKind::ForeignMeet,
            format!("expected meet {meet_id}, found {published_meet}"),
        ));
    }
    let gender = Gender::parse_milesplit(row.gender);
    if gender == Gender::Unknown || matches!(event_kind, EventKind::Unmapped { .. }) {
        return Err((
            OwnedRejectionKind::InvalidContext,
            "unknown gender or event".to_string(),
        ));
    }
    let (mark, timing) = match row
        .status_code
        .and_then(crate::result_status::invalid_token)
    {
        Some(status) => (Mark::Raw(status.into()), None),
        None => parsed_mark(&event_kind, row.mark)?,
    };
    let (grad_year, cohort) = cohort(row.grad_year.as_ref());
    Ok(OwnedPerformance {
        locator,
        result_id: provider_id(&row.id)?,
        meet_id: published_meet,
        result_set_id: provider_id(&row.meet_results_id)?,
        source_athlete,
        team_id: provider_id(&row.team_id)?,
        first_name: row.first_name.to_string(),
        last_name: row.last_name.to_string(),
        gender,
        grad_year,
        cohort,
        event_kind,
        mark,
        timing,
        provider: value,
    })
}

fn provider_id(value: &Scalar<'_>) -> Result<u64, (OwnedRejectionKind, String)> {
    value.id().ok_or_else(|| {
        (
            OwnedRejectionKind::InvalidIdentity,
            "missing or noncanonical positive provider ID".to_string(),
        )
    })
}

fn validate_name(row: &WirePerformance<'_>) -> Result<(), (OwnedRejectionKind, String)> {
    if (row.first_name.trim().is_empty() && row.last_name.trim().is_empty())
        || row
            .first_name
            .chars()
            .chain(row.last_name.chars())
            .any(char::is_control)
    {
        return Err((
            OwnedRejectionKind::InvalidContext,
            "missing or malformed published person name".to_string(),
        ));
    }
    Ok(())
}

fn owner(row: &WirePerformance<'_>) -> Result<SourceIdentity, (OwnedRejectionKind, String)> {
    let raw = row.athlete_id.as_ref().ok_or_else(|| {
        (
            OwnedRejectionKind::MissingOwner,
            "no published athleteId".to_string(),
        )
    })?;
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, raw.text());
    let (_, id) = person_key(&source).ok_or_else(|| {
        (
            OwnedRejectionKind::InvalidIdentity,
            "invalid or placeholder athleteId".to_string(),
        )
    })?;
    let profile = row.profile_url.ok_or_else(|| {
        (
            OwnedRejectionKind::MissingOwner,
            "no published profileUrl".to_string(),
        )
    })?;
    if profile_id(profile) != Some(id) {
        return Err((
            OwnedRejectionKind::ProfileMismatch,
            "profileUrl disagrees with athleteId or is not a public MileSplit athlete URL"
                .to_string(),
        ));
    }
    Ok(source.with_url(profile))
}

fn profile_id(profile: &str) -> Option<u64> {
    let url = url::Url::parse(profile).ok()?;
    let host = url.host_str()?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || (host != "milesplit.com" && !host.ends_with(".milesplit.com"))
    {
        return None;
    }
    let mut segments = url.path_segments()?;
    if segments.next()? != "athletes" {
        return None;
    }
    let token = segments.next()?.split('-').next()?;
    let identity = SourceIdentity::new(SourceNamespace::MilesplitAthlete, token);
    person_key(&identity).map(|(_, id)| id)
}

fn cohort(raw: Option<&Scalar<'_>>) -> (Option<GradYear>, OwnedCohort) {
    let Some(raw) = raw else {
        return (None, OwnedCohort::Missing);
    };
    let text = raw.text();
    if text.is_empty() || text == "0" {
        return (None, OwnedCohort::Missing);
    }
    match text.parse::<i16>().ok().and_then(GradYear::new) {
        Some(year) => (Some(year), OwnedCohort::Published),
        None => (None, OwnedCohort::Invalid),
    }
}

pub(super) fn no_mark(raw: &str) -> bool {
    crate::result_status::invalid_token(raw).is_some()
}

fn parsed_mark(
    kind: &EventKind,
    raw: &str,
) -> Result<(Mark, Option<census_domain::model::TimingMethod>), (OwnedRejectionKind, String)> {
    if no_mark(raw) {
        return Ok((Mark::Raw(raw.to_string()), None));
    }
    let parsed = match kind {
        EventKind::Decathlon | EventKind::Pentathlon | EventKind::Heptathlon => {
            crate::hytek::parse_points(raw).map(|mark| (mark, None))
        }
        kind if kind.is_field() => crate::milesplit::mark::parse_published_metric_distance(raw)
            .map(Mark::DistanceMetres)
            .or_else(|| crate::hytek::parse_field_mark(raw))
            .map(|mark| (mark, None)),
        _ => crate::milesplit::mark::parse_published_time(raw)
            .map(|(time, timing)| (Mark::TimeSeconds(time), timing)),
    };
    parsed.ok_or_else(|| {
        (
            OwnedRejectionKind::InvalidContext,
            "malformed published mark".to_string(),
        )
    })
}
