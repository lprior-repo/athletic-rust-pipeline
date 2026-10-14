use super::super::super::cells::{row, Cell};
use super::super::columns::{flag, published};
use super::rules::PR_EVENTS;
use crate::bests::SharedSelection;
use crate::workbook::recruiting::profiles::Profiles;
use crate::workbook::ReportResult;
use census_domain::model::performance_comparison::SurfaceClass;
use census_domain::model::{CanonicalAthlete, EventKind, Sport};

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
    let events: std::collections::BTreeMap<_, _> = prs
        .iter()
        .map(|pr| {
            let event = if pr.key.surface == SurfaceClass::CrossCountry {
                std::borrow::Cow::Borrowed("CrossCountry")
            } else {
                pr.key.event_kind.stable_key()
            };
            (event, *pr)
        })
        .collect();
    let names: Vec<_> = events
        .iter()
        .map(|(event, pr)| pr_mark_name(pr, event))
        .collect();
    vec![Cell::text(names.join("; "))]
}

pub(super) fn headline_pr_summary(
    _athlete: &CanonicalAthlete,
    prs: &[&SharedSelection],
) -> Vec<Cell> {
    vec![qualified_summary(prs.iter().copied())]
}

pub(super) fn pr_event_cells(prs: &[&SharedSelection]) -> Vec<Cell> {
    PR_EVENTS
        .iter()
        .map(|event| event_cell(prs, event))
        .collect()
}

fn event_cell(prs: &[&SharedSelection], event: &str) -> Cell {
    qualified_summary(
        prs.iter()
            .copied()
            .filter(|pr| selection_matches_event(pr, event)),
    )
}

fn selection_matches_event(pr: &SharedSelection, event: &str) -> bool {
    match event {
        "CrossCountry" => pr.key.surface == SurfaceClass::CrossCountry,
        "Track5000m" => {
            pr.key.surface != SurfaceClass::CrossCountry
                && pr.key.event_kind == EventKind::Track5000m
        }
        _ => pr.key.event_kind.stable_key() == event,
    }
}

fn pr_mark_name<'a>(pr: &SharedSelection, event: &'a str) -> &'a str {
    if pr.key.surface == SurfaceClass::CrossCountry {
        "XC"
    } else {
        pr_event_name(event)
    }
}

const SUMMARY_OVERFLOW: &str = "; additional classified marks in PRs";
const SUMMARY_LIMIT: usize = 32_767;

fn qualified_summary<'a>(mut prs: impl Iterator<Item = &'a SharedSelection>) -> Cell {
    let result = prs.try_fold((String::new(), 0_usize, 0_usize), append_summary_mark);
    match result {
        Ok((text, _, _)) if text.is_empty() => Cell::Empty,
        Ok((text, _, _)) => Cell::text(text),
        Err(mut text) => {
            text.push_str(SUMMARY_OVERFLOW);
            Cell::text(text)
        }
    }
}

fn append_summary_mark(
    (mut text, used, marker_end): (String, usize, usize),
    pr: &SharedSelection,
) -> Result<(String, usize, usize), String> {
    let start = text.len();
    if !text.is_empty() {
        text.push_str("; ");
    }
    append_qualified_mark(&mut text, pr);
    let required = text
        .get(start..)
        .map_or(0, |mark| mark.encode_utf16().count());
    let total = used.saturating_add(required);
    if total > SUMMARY_LIMIT {
        text.truncate(marker_end);
        return Err(text);
    }
    let marker_end = if total <= SUMMARY_LIMIT.saturating_sub(SUMMARY_OVERFLOW.len()) {
        text.len()
    } else {
        marker_end
    };
    Ok((text, total, marker_end))
}

fn append_qualified_mark(text: &mut String, pr: &SharedSelection) {
    text.push_str(pr_mark_name(pr, &pr.key.event_kind.stable_key()));
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
    if let Some(context) = crate::bests::context::cross_country(pr) {
        text.push_str(", ");
        text.push_str(&context);
    }
    append_specification(text, &pr.source.specification);
    append_adjustment(text, pr);
    text.push(']');
}

fn append_adjustment(text: &mut String, pr: &SharedSelection) {
    if pr.result.adjustment_kind.is_none() {
        return;
    }
    text.push_str(", conversion ");
    match pr.result.adjustment_basis {
        Some(basis) => text.push_str(basis.label()),
        None => text.push_str("unknown"),
    }
    if let Some(original) = &pr.result.original_mark {
        text.push_str(" from ");
        text.push_str(&crate::bests::mark_text(original));
    }
    if let Some(note) = &pr.result.adjustment_note {
        text.push_str(": ");
        text.push_str(note);
    }
}

fn append_specification(
    text: &mut String,
    specification: &census_domain::model::EventSpecification,
) {
    if let Some(mass) = specification.implement {
        text.push_str(", implement ");
        text.push_str(&mass.micrograms().to_string());
        text.push_str(" ug");
    }
    if let Some(hurdles) = specification.hurdles {
        text.push_str(", hurdle height ");
        text.push_str(&hurdles.height_micrometres.to_string());
        text.push_str(" um");
        if let Some(spacing) = hurdles.spacing_micrometres {
            text.push_str(", hurdle spacing ");
            text.push_str(&spacing.to_string());
            text.push_str(" um");
        }
    }
    if let Some(track) = specification.indoor_track {
        text.push_str(", indoor track length ");
        text.push_str(&track.length_micrometres.to_string());
        text.push_str(" um, banking ");
        text.push_str(match track.banking {
            census_domain::model::TrackBanking::Flat => "flat",
            census_domain::model::TrackBanking::Banked => "banked",
            census_domain::model::TrackBanking::Unknown => "unknown",
        });
    }
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
        "Track300m" => "300m",
        "Track400m" => "400m",
        "Track500m" => "500m",
        "Track600m" => "600m",
        "Track800m" => "800m",
        "Track1600m" => "1600m",
        "Track2000m" => "2000m",
        "Track3200m" => "3200m",
        "Track1Mile" => "1 Mile",
        "Track2Miles" => "2 Miles",
        "Track5000m" => "5000m",
        "Track10000m" => "10000m",
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
