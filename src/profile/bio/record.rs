//! Single result record: mark, joins, timing, and best claims.

use serde_json::{Map, Value};

use crate::domain::evidence::{EvidenceIssue, ResultEvidence, Sport};

use super::distances::distance_join;
use super::events;
use super::fields::{
    best_claim, bounded_id, display_value, ev, issue, optional_display, optional_text,
    required_u16, required_u64, timing, valid_short_code,
};
use super::relays::attribution;
use super::results::ResultContext;

pub(super) fn parse_result(
    item: &Value,
    index: usize,
    context: &ResultContext<'_>,
    issues: &mut Vec<EvidenceIssue>,
) -> Option<ResultEvidence> {
    let Some(obj) = item.as_object() else {
        issues.push(issue(
            "unknown_result_shape",
            "result is not an object",
            Some(ev(context.digest, format!("/results/{index}"))),
        ));
        return None;
    };
    let key = if context.sport == Sport::TrackField {
        "resultsTF"
    } else {
        "resultsXC"
    };
    let locator = format!("/{key}/{index}");
    let joins = join_fields(obj, &locator, context, issues);
    let (event_id, event_name, event_description, event_type, units, personal_event) =
        event_info(obj, context, &locator, issues);
    let mark = mark_field(obj, &locator, context, issues);
    let short_code = short_code_field(obj, &locator, context, issues);
    let result_url = result_link(&short_code, joins.reported, personal_event, joins.meet_id);
    Some(ResultEvidence {
        result_id: joins.result_id,
        sport: context.sport,
        event_id,
        event_name,
        event_description,
        event_type,
        mark,
        units,
        season: joins.season,
        team_id: joins.team_id,
        meet_id: joins.meet_id,
        meet_name: optional_text(obj.get("meetName"))
            .or_else(|| context.meets.get(&joins.meet_id).cloned().flatten()),
        date: optional_text(obj.get("ResultDate")),
        wind: optional_display(obj.get("Wind")),
        timing: timing(obj.get("FAT")),
        personal_best: best_claim(obj.get("PersonalBest"), context.sport),
        season_best: best_claim(obj.get("SeasonBest"), context.sport),
        attribution: attribution(
            context.athlete,
            joins.reported,
            personal_event,
            context.relays,
        ),
        short_code,
        result_url,
        evidence: ev(context.digest, locator),
    })
}

/// Result identifiers plus the team and meet joins they must satisfy.
struct ResultJoins {
    result_id: u64,
    reported: Option<u64>,
    season: u16,
    team_id: u64,
    meet_id: u64,
}

fn join_fields(
    obj: &Map<String, Value>,
    locator: &str,
    context: &ResultContext<'_>,
    issues: &mut Vec<EvidenceIssue>,
) -> ResultJoins {
    let result_id = required_u64(obj, "IDResult", locator, context.digest, issues);
    let reported = obj
        .get("AthleteID")
        .and_then(Value::as_u64)
        .filter(|value| bounded_id(*value));
    let season = required_u16(obj, "SeasonID", locator, context.digest, issues);
    let team_id = bounded_u64(obj, "SchoolID");
    let meet_id = bounded_u64(obj, "MeetID");
    join_issues(team_id, meet_id, locator, context, issues);
    ResultJoins {
        result_id,
        reported,
        season,
        team_id,
        meet_id,
    }
}

fn bounded_u64(obj: &Map<String, Value>, key: &str) -> u64 {
    obj.get(key)
        .and_then(Value::as_u64)
        .filter(|value| bounded_id(*value))
        .map_or(0, |value| value)
}

fn join_issues(
    team_id: u64,
    meet_id: u64,
    locator: &str,
    context: &ResultContext<'_>,
    issues: &mut Vec<EvidenceIssue>,
) {
    if team_id == 0 || !context.teams.contains(&team_id) {
        issues.push(issue(
            "missing_team_join",
            "result SchoolID does not join allTeams",
            Some(ev(context.digest, locator.to_owned())),
        ));
    }
    if meet_id == 0 || !context.meets.contains_key(&meet_id) {
        issues.push(issue(
            "missing_meet_join",
            "result MeetID does not join meets",
            Some(ev(context.digest, locator.to_owned())),
        ));
    }
}

fn mark_field(
    obj: &Map<String, Value>,
    locator: &str,
    context: &ResultContext<'_>,
    issues: &mut Vec<EvidenceIssue>,
) -> String {
    obj.get("Result")
        .filter(|value| !value.is_null())
        .map(display_value)
        .unwrap_or_else(|| {
            issues.push(issue(
                "missing_result_mark",
                "result has no Result display mark",
                Some(ev(context.digest, locator.to_owned())),
            ));
            String::new()
        })
}

fn short_code_field(
    obj: &Map<String, Value>,
    locator: &str,
    context: &ResultContext<'_>,
    issues: &mut Vec<EvidenceIssue>,
) -> Option<String> {
    let short_code = obj
        .get("shortCode")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| valid_short_code(value))
        .map(str::to_owned);
    if short_code.is_none() {
        issues.push(issue(
            "missing_short_code",
            "result has no valid shortCode",
            Some(ev(context.digest, locator.to_owned())),
        ));
    }
    short_code
}

/// Only a reported athlete in a personal event at a known meet gets a result link.
fn result_link(
    short_code: &Option<String>,
    reported: Option<u64>,
    personal_event: bool,
    meet_id: u64,
) -> Option<String> {
    short_code
        .as_ref()
        .filter(|_| reported.is_some_and(|value| value > 0) && personal_event && meet_id > 0)
        .map(|code| format!("http://www.athletic.net/result/{code}"))
}
fn event_info(
    obj: &Map<String, Value>,
    context: &ResultContext<'_>,
    locator: &str,
    issues: &mut Vec<EvidenceIssue>,
) -> (
    Option<u64>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
) {
    match context.sport {
        Sport::TrackField => {
            let id = obj
                .get("EventID")
                .and_then(Value::as_u64)
                .filter(|value| bounded_id(*value));
            let event = id.and_then(|id| events::lookup(context.events, id, obj));
            match event {
                Some(event) => (
                    id,
                    event.name.clone(),
                    event.description.clone(),
                    event.kind.clone(),
                    event.units.clone(),
                    event.personal,
                ),
                None => {
                    issues.push(issue(
                        "missing_event_join",
                        "TF result event and variant do not uniquely join eventsTF",
                        Some(ev(context.digest, locator)),
                    ));
                    (id, "Unknown event".to_owned(), None, None, None, false)
                }
            }
        }
        Sport::CrossCountry => {
            let id = obj
                .get("Distance")
                .and_then(Value::as_u64)
                .filter(|value| bounded_id(*value));
            let Some(id) = id else {
                issues.push(issue(
                    "missing_distance_join",
                    "XC result has no Distance",
                    Some(ev(context.digest, locator)),
                ));
                return (None, "Unknown distance".to_owned(), None, None, None, true);
            };
            let Some((name, units)) = distance_join(id, context, locator, issues) else {
                return (None, "Unknown distance".to_owned(), None, None, None, true);
            };
            (None, name, None, None, units, true)
        }
    }
}

pub(super) fn result_size(result: &ResultEvidence) -> Option<usize> {
    [
        Some(result.event_name.as_str()),
        result.event_description.as_deref(),
        result.event_type.as_deref(),
        Some(result.mark.as_str()),
        result.units.as_deref(),
        result.meet_name.as_deref(),
        result.date.as_deref(),
        result.wind.as_deref(),
        result.timing.as_deref(),
        result.short_code.as_deref(),
        result.result_url.as_deref(),
        Some(result.evidence.locator.as_str()),
        Some(result.evidence.document.as_str()),
    ]
    .into_iter()
    .flatten()
    .try_fold(std::mem::size_of::<ResultEvidence>(), |bytes, text| {
        bytes.checked_add(text.len())
    })
}
