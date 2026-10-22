use crate::bests::mark_text;
use crate::workbook::reported::Projection;
use census_domain::model::performance_comparison::SurfaceClass;
use census_domain::model::{CanonicalAthlete, EventKind};
use std::collections::BTreeSet;

pub(super) fn values(
    projection: &Projection<'_>,
    athlete: &CanonicalAthlete,
    schema: crate::workbook::verify::labels::athlete_headers::Schema,
) -> Vec<crate::workbook::verify::canonical::Value> {
    let bests = projection.bests_for(&athlete.id).collect::<Vec<_>>();
    let mut event_names = BTreeSet::new();
    for reported in &bests {
        event_names.insert(event_name(reported));
    }
    let event_list = event_names.into_iter().collect::<Vec<_>>().join("; ");
    let headline = summary(&bests, None);
    let pr_values: Vec<_> = schema
        .pr_events()
        .iter()
        .map(|event| crate::workbook::verify::canonical::Value::text(summary(&bests, Some(*event))))
        .collect();
    let mut result = vec![
        crate::workbook::verify::canonical::Value::text(event_list),
        crate::workbook::verify::canonical::Value::text(headline),
    ];
    result.extend(pr_values);
    result
}

fn summary(bests: &[&crate::workbook::reported::ReportedBest<'_>], event: Option<&str>) -> String {
    let parts: Vec<String> = bests
        .iter()
        .filter(|reported| event.is_none_or(|e| matches_event(reported, e)))
        .map(|reported| format_best(reported))
        .collect();
    parts.join("; ")
}

fn format_best(reported: &crate::workbook::reported::ReportedBest<'_>) -> String {
    let key = &reported.group.key;
    let mark = match reported.performance {
        Some(performance) => mark_text(&performance.mark),
        None => "no comparable candidate".to_owned(),
    };
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
        context,
    )
}

fn event_name(reported: &crate::workbook::reported::ReportedBest<'_>) -> String {
    if reported.group.key.surface == SurfaceClass::CrossCountry {
        "CrossCountry".to_owned()
    } else {
        reported.group.key.event_kind.stable_key().into_owned()
    }
}

fn matches_event(reported: &crate::workbook::reported::ReportedBest<'_>, event: &str) -> bool {
    match event {
        "CrossCountry" => reported.group.key.surface == SurfaceClass::CrossCountry,
        "Track5000m" => {
            reported.group.key.surface != SurfaceClass::CrossCountry
                && reported.group.key.event_kind == EventKind::Track5000m
        }
        _ => reported.group.key.event_kind.stable_key() == event,
    }
}
