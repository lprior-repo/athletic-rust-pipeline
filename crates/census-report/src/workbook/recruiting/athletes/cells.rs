//! Pure cell helpers — no `Dataset` dependency.

use super::super::super::cells::{row, Cell};
use super::super::columns::{flag, published};
use super::rules::PR_EVENTS;
use crate::bests::{mark_text, mark_value};
use crate::workbook::recruiting::profiles::Profiles;
use crate::workbook::recruiting::prs::PrRow;
use crate::workbook::ReportResult;
use census_domain::model::{CanonicalAthlete, Sport};

/// The XC, Indoor, and Outdoor sport-flag columns.
pub(super) fn participation_flags(athlete: &CanonicalAthlete) -> Vec<Cell> {
    row!(
        flag(athlete.sports.contains(&Sport::CrossCountry)),
        flag(athlete.sports.contains(&Sport::IndoorTrack)),
        flag(athlete.sports.contains(&Sport::OutdoorTrack)),
    )
}

/// `TF` — `yes` when the athlete's sports include `IndoorTrack` or `OutdoorTrack`, blank otherwise.
pub(super) fn tf_flag(athlete: &CanonicalAthlete) -> Vec<Cell> {
    vec![flag(athlete.sports.iter().any(|sport| {
        matches!(sport, Sport::IndoorTrack | Sport::OutdoorTrack)
    }))]
}

/// The distinct event names the athlete has a stored performance in, in canonical order,
/// joined with `; `. Blank when none.
pub(super) fn event_list(_athlete: &CanonicalAthlete, prs: &[&PrRow]) -> Vec<Cell> {
    if prs.is_empty() {
        return vec![Cell::Empty];
    }
    // Collect unique events in canonical PR_EVENTS order
    let mut event_set = std::collections::BTreeSet::new();
    for pr in prs {
        event_set.insert(&pr.event);
    }
    let events: Vec<String> = PR_EVENTS
        .iter()
        .filter(|e| event_set.contains(&e.to_string()))
        .map(|e| pr_event_name(e))
        .collect();
    vec![Cell::text(events.join("; "))]
}

/// The athlete's PRs in canonical event order, formatted `Event Mark`, joined with `; `,
/// capped at 10 entries with a trailing `...` when more exist. Blank when none.
pub(super) fn headline_pr_summary(_athlete: &CanonicalAthlete, prs: &[&PrRow]) -> Vec<Cell> {
    if prs.is_empty() {
        return vec![Cell::Empty];
    }
    // Build a map from event key to mark, then iterate in canonical order
    let mut mark_map: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
    for pr in prs {
        mark_map.entry(pr.event.as_str()).or_insert_with(|| {
            format!(
                "{} {}",
                pr_event_name(&pr.event),
                mark_text(&pr.source_mark)
            )
        });
    }
    let entries: Vec<String> = PR_EVENTS
        .iter()
        .filter_map(|e| mark_map.get(*e))
        .cloned()
        .collect();
    let truncated = if entries.len() > 10 {
        let truncated: Vec<String> = entries.into_iter().take(10).collect();
        truncated.join("; ") + "; ..."
    } else {
        entries.join("; ")
    };
    vec![Cell::text(truncated)]
}

/// The nineteen supported per-event PR columns, in canonical order.
pub(super) fn pr_event_cells(prs: &[&PrRow]) -> Vec<Cell> {
    PR_EVENTS
        .iter()
        .map(|event| {
            prs.iter()
                .find(|pr| pr.event == *event)
                .and_then(|pr| mark_value(&pr.source_mark))
                .map_or(Cell::Empty, Cell::Number)
        })
        .collect()
}

/// The `Performance count` and `Meet count` columns.
pub(super) fn participation_metrics(
    tally: Option<&crate::workbook::recruiting::facts::AthleteTally>,
) -> ReportResult<Vec<Cell>> {
    Ok(vec![
        Cell::number(tally.map_or(0, |t| t.performances))?,
        Cell::number(tally.map_or(0, |t| t.meets.len()))?,
    ])
}

/// The three profile-URL columns, in the order [`Profiles`] splits them.
pub(super) fn profile_cells(profiles: Profiles) -> Vec<Cell> {
    row!(
        published(profiles.athletic_net),
        published(profiles.milesplit),
        Cell::text(profiles.other.join("; ")),
    )
}

/// `Public Recruiting GPA` — always blank. `census-domain` has no GPA observation entity,
/// objective §36 forbids inferring one.
pub(super) fn public_recruiting_gpa() -> Vec<Cell> {
    vec![Cell::Empty]
}

/// `GPA Source` — always blank. A GPA may only appear next to a `GPA Source` naming where it came from.
pub(super) fn gpa_source() -> Vec<Cell> {
    vec![Cell::Empty]
}

/// Derive a human-readable event name from a `PR_EVENTS` key.
pub(super) fn pr_event_name(key: &str) -> String {
    let name = match key {
        "Track100m" => "100m",
        "Track200m" => "200m",
        "Track400m" => "400m",
        "Track800m" => "800m",
        "Track1600m" => "1600m",
        "Track3200m" => "3200m",
        "Track1Mile" => "1 Mile",
        "Track5000m" => "5000m",
        "Track100mHurdles" => "100m H",
        "Track110mHurdles" => "110m H",
        "Track300mHurdles" => "300m H",
        "HighJump" => "High Jump",
        "LongJump" => "Long Jump",
        "TripleJump" => "Triple Jump",
        "PoleVault" => "Pole Vault",
        "ShotPut" => "Shot Put",
        "Discus" => "Discus",
        "Javelin" => "Javelin",
        "CrossCountry" => "XC",
        other => other,
    };
    name.to_string()
}
