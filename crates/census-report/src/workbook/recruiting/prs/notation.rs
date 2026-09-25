//! The rendering helpers the `PRs` reduction shares: how one slot is keyed, which URL a row
//! publishes, and how a meet whose rows disagree is written.

use census_domain::model::{CanonicalMeet, CanonicalPerformance, EventKind, Sport};
use std::collections::BTreeSet;

/// The slot one performance competes in: the event, the season it belongs to, and — for an outdoor
/// event with a wind reading past the legal limit — the wind-assisted variant, which never displaces
/// a legal mark.
pub(super) fn slot_key(
    kind: &EventKind,
    meet: Option<&CanonicalMeet>,
    wind_mps: Option<f64>,
) -> String {
    let (i, o) = meet.map_or((false, false), |m| {
        (
            m.sports.contains(&Sport::IndoorTrack),
            m.sports.contains(&Sport::OutdoorTrack),
        )
    });
    let b = kind.stable_key().into_owned();
    match (i, o, wind_mps) {
        (true, _, _) => format!("{b}:indoor"),
        (_, true, Some(w)) if w > 2.0 => format!("{b}:wa"),
        _ => b,
    }
}

/// The winning performance's published address, or nothing when no source recorded one.
pub(super) fn result_url(performance: &CanonicalPerformance) -> String {
    performance
        .evidence
        .iter()
        .find_map(|evidence| evidence.source.url.clone())
        .unwrap_or_default()
}

/// One meet's conflicting published marks, as the `PR conflict` cell writes them.
pub(super) fn disagreement(
    meet: &str,
    marks: &BTreeSet<String>,
    meets: &std::collections::BTreeMap<String, CanonicalMeet>,
) -> String {
    let name = meets.get(meet).map_or(meet, |row| row.name.as_str());
    let published: Vec<&str> = marks.iter().map(String::as_str).collect();
    format!("{name}: {}", published.join(" | "))
}
