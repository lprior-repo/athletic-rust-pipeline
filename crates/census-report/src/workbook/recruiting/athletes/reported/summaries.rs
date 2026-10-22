use super::*;
use crate::workbook::reported::ReportedBest;
use census_domain::model::performance_comparison::SurfaceClass;

pub(super) fn cells(projection: &Projection<'_>, athlete: &CanonicalAthlete) -> Vec<Cell> {
    let groups = projection.bests_for(&athlete.id).collect::<Vec<_>>();
    let names = groups
        .iter()
        .map(|reported| event_name(reported))
        .collect::<BTreeSet<_>>();
    let mut result = vec![text(names.into_iter().collect::<Vec<_>>().join("; "))];
    result.push(summary(&groups, None));
    result.extend(super::super::rules::PR_EVENTS.map(|event| summary(&groups, Some(event))));
    result
}

fn summary(groups: &[&ReportedBest<'_>], event: Option<&str>) -> Cell {
    text(
        groups
            .iter()
            .copied()
            .filter(|reported| event.is_none_or(|event| matches_event(reported, event)))
            .map(|reported| {
                let key = &reported.group.key;
                let mark = reported.performance.map_or_else(
                    || "no comparable candidate".to_owned(),
                    |performance| crate::bests::mark_text(&performance.mark),
                );
                let context = key
                    .context
                    .as_ref()
                    .map_or_else(String::new, |id| format!(", event {}", id.as_str()));
                format!(
                    "{} {} [{}, {}, {}{}]",
                    event_name(reported),
                    mark,
                    key.surface.label(),
                    key.wind_class.label(),
                    key.timing.label(),
                    context
                )
            })
            .collect::<Vec<_>>()
            .join("; "),
    )
}

fn event_name(reported: &ReportedBest<'_>) -> String {
    if reported.group.key.surface == SurfaceClass::CrossCountry {
        "CrossCountry".to_owned()
    } else {
        reported.group.key.event_kind.stable_key().into_owned()
    }
}

fn matches_event(reported: &ReportedBest<'_>, event: &str) -> bool {
    match event {
        "CrossCountry" => reported.group.key.surface == SurfaceClass::CrossCountry,
        "Track5000m" => {
            reported.group.key.surface != SurfaceClass::CrossCountry
                && reported.group.key.event_kind == census_domain::model::EventKind::Track5000m
        }
        _ => reported.group.key.event_kind.stable_key() == event,
    }
}

fn text(value: String) -> Cell {
    if value.is_empty() {
        Cell::Empty
    } else {
        Cell::text(value)
    }
}
