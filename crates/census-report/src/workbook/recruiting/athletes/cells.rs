
use super::super::super::cells::{row, Cell};
use super::super::columns::{flag, published};
use super::rules::PR_EVENTS;
use crate::bests::SharedSelection;
use crate::workbook::recruiting::profiles::Profiles;
use crate::workbook::ReportResult;
use census_domain::model::{CanonicalAthlete, Sport};

pub(super) fn participation_flags(athlete: &CanonicalAthlete) -> Vec<Cell> {
    row!(
        flag(athlete.sports.contains(&Sport::CrossCountry)),
        flag(athlete.sports.contains(&Sport::IndoorTrack)),
        flag(athlete.sports.contains(&Sport::OutdoorTrack)),
    )
}

pub(super) fn tf_flag(athlete: &CanonicalAthlete) -> Vec<Cell> {
    vec![flag(athlete.sports.iter().any(|sport| {
        matches!(sport, Sport::IndoorTrack | Sport::OutdoorTrack)
    }))]
}

pub(super) fn event_list(_athlete: &CanonicalAthlete, prs: &[&SharedSelection]) -> Vec<Cell> {
    if prs.is_empty() {
        return vec![Cell::Empty];
    }
    let events: std::collections::BTreeSet<_> = prs.iter()
        .map(|pr| pr.key.event_kind.stable_key())
        .collect();
    let names: Vec<_> = events.iter().map(|event| pr_event_name(event)).collect();
    vec![Cell::text(names.join("; "))]
}

pub(super) fn headline_pr_summary(_athlete: &CanonicalAthlete, prs: &[&SharedSelection]) -> Vec<Cell> {
    if prs.is_empty() {
        return vec![Cell::Empty];
    }
    let mut text = String::new();
    for (index, pr) in prs.iter().take(10).enumerate() {
        if index != 0 {
            text.push_str("; ");
        }
        append_qualified_mark(&mut text, pr);
    }
    if prs.len() > 10 {
        text.push_str("; additional classified marks in PRs");
    }
    vec![Cell::text(text)]
}

pub(super) fn pr_event_cells(prs: &[&SharedSelection]) -> Vec<Cell> {
    PR_EVENTS.iter().map(|event| event_cell(prs, event)).collect()
}

fn event_cell(prs: &[&SharedSelection], event: &str) -> Cell {
    let mut selected = prs.iter().copied()
        .filter(|pr| pr.key.event_kind.stable_key() == event);
    let Some(first) = selected.next() else {
        return Cell::Empty;
    };
    let Some(second) = selected.next() else {
        return first.normalized.map_or_else(|| Cell::text(first.mark_text()), Cell::Number);
    };
    let mut text = String::new();
    append_qualified_mark(&mut text, first);
    for pr in std::iter::once(second).chain(selected) {
        text.push_str("; ");
        append_qualified_mark(&mut text, pr);
    }
    Cell::text(text)
}

fn append_qualified_mark(text: &mut String, pr: &SharedSelection) {
    text.push_str(pr_event_name(&pr.key.event_kind.stable_key()));
    text.push(' ');
    text.push_str(&pr.mark_text());
    text.push_str(" [");
    text.push_str(pr.key.surface.label());
    text.push_str(", ");
    text.push_str(pr.key.wind_class.label());
    text.push_str(", ");
    text.push_str(pr.key.timing.label());
    if let Some(context) = &pr.key.context {
        text.push_str(", event ");
        text.push_str(context.as_str());
    }
    text.push(']');
}

pub(super) fn participation_metrics(
    tally: Option<&crate::workbook::recruiting::facts::AthleteTally>,
) -> ReportResult<Vec<Cell>> {
    Ok(vec![
        Cell::number(tally.map_or(0, |t| t.performances))?,
        Cell::number(tally.map_or(0, |t| t.meets.len()))?,
    ])
}

pub(super) fn profile_cells(profiles: Profiles) -> Vec<Cell> {
    row!(
        published(profiles.athletic_net),
        published(profiles.milesplit),
        Cell::text(profiles.other.join("; ")),
    )
}

pub(super) fn public_recruiting_gpa() -> Vec<Cell> {
    vec![Cell::Empty]
}

pub(super) fn gpa_source() -> Vec<Cell> {
    vec![Cell::Empty]
}

pub(super) fn pr_event_name(key: &str) -> &str {
    match key {
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
    }
}
