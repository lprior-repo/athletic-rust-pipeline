use std::collections::{BTreeMap, BTreeSet};

use crate::bests::SharedSelection;
use crate::export::ReportedProfilePrTable;
use crate::workbook::reported_profile::{self, GapLine};
use census_domain::model::performance_comparison::SurfaceClass;
use census_domain::model::{CanonicalAthlete, EventKind, ObservedGrade, SchoolYear};

use super::super::canonical::Value;
use super::super::labels;
use super::super::labels::athlete_headers::Schema;

pub(super) fn newest_observation(athlete: &CanonicalAthlete) -> Option<&ObservedGrade> {
    athlete
        .observed_grades
        .iter()
        .max_by_key(|grade| (grade.school_year.get(), grade.grade.get()))
}

pub(in crate::workbook::verify) fn observed_season(
    athlete: &CanonicalAthlete,
) -> Option<SchoolYear> {
    newest_observation(athlete)
        .map(|grade| grade.school_year)
        .or_else(|| {
            athlete
                .evidence
                .iter()
                .filter_map(|evidence| SchoolYear::from_date(&evidence.observed_on))
                .max_by_key(|year| year.get())
        })
}

pub(super) fn flag(recorded: bool) -> Value {
    Value::flagged(recorded)
}

pub(in crate::workbook::verify) fn source_count(athlete: &CanonicalAthlete) -> usize {
    athlete
        .identities()
        .map(|identity| &identity.namespace)
        .collect::<BTreeSet<_>>()
        .len()
}

#[derive(Default)]
pub(in crate::workbook::verify) struct Profiles {
    pub(in crate::workbook::verify) athletic_net: Option<String>,
    pub(in crate::workbook::verify) milesplit: Option<String>,
    pub(in crate::workbook::verify) other: Vec<String>,
}

pub(in crate::workbook::verify) fn profiles_of(athlete: &CanonicalAthlete) -> Profiles {
    let mut profiles = Profiles::default();
    let mut seen: Vec<String> = Vec::new();
    let candidates = athlete.public_profile_urls.iter().cloned().chain(
        athlete
            .identities()
            .filter_map(|identity| identity.url.clone()),
    );
    for url in candidates {
        if url.is_empty() || seen.contains(&url) {
            continue;
        }
        seen.push(url.clone());
        classify_profile(&mut profiles, url);
    }
    profiles
}

fn classify_profile(profiles: &mut Profiles, url: String) {
    let lowered = url.to_ascii_lowercase();
    match lowered.as_str() {
        _ if lowered.contains("athletic.net") && profiles.athletic_net.is_none() => {
            profiles.athletic_net = Some(url);
        }
        _ if lowered.contains("milesplit") && profiles.milesplit.is_none() => {
            profiles.milesplit = Some(url);
        }
        _ => profiles.other.push(url),
    }
}

pub(super) fn reported_gaps(
    prs: &[&SharedSelection],
    schema: Schema,
    tables: &[&ReportedProfilePrTable],
) -> Vec<GapLine> {
    let covered = reported_profile::covered_events(schema.pr_events(), |event| {
        prs.iter().any(|pr| selection_matches_event(pr, event))
    });
    reported_profile::gap_lines(&covered, tables)
}

pub(super) fn event_list(prs: &[&SharedSelection], schema: Schema, gaps: &[GapLine]) -> Value {
    if prs.is_empty() && gaps.is_empty() {
        return Value::Empty;
    }
    let events: BTreeMap<_, _> = prs
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
    let mut names: Vec<String> = events
        .iter()
        .map(|(event, pr)| labels::mark_name(pr, event, schema).to_owned())
        .collect();
    names.extend(
        gaps.iter()
            .map(|gap| reported_profile::display(gap.event_key)),
    );
    names.sort();
    names.dedup();
    Value::text(names.join("; "))
}

pub(super) fn headline(prs: &[&SharedSelection], schema: Schema, gaps: &[GapLine]) -> Value {
    merge_gaps(qualified_summary(prs.iter().copied(), schema), gaps)
}

fn merge_gaps(headline: Value, gaps: &[GapLine]) -> Value {
    if gaps.is_empty() {
        return headline;
    }
    let mut text = match headline {
        Value::Text(text) => text,
        Value::Empty => String::new(),
        other => return other,
    };
    let mut used = text.encode_utf16().count();
    let mut end = text.len();
    for gap in gaps {
        let separator = if text.is_empty() { 0 } else { 2 };
        let needed = gap.line.encode_utf16().count().saturating_add(separator);
        if used.saturating_add(needed) > 32_767 {
            text.truncate(end);
            text.push_str(SUMMARY_OVERFLOW);
            return Value::text(text);
        }
        if separator != 0 {
            text.push_str("; ");
        }
        text.push_str(gap.line.as_str());
        used = used.saturating_add(needed);
        end = text.len();
    }
    Value::text(text)
}

pub(super) fn pr_events(
    prs: &[&SharedSelection],
    schema: Schema,
    gaps: &BTreeMap<&str, &str>,
) -> Vec<Value> {
    schema
        .pr_events()
        .iter()
        .map(|event| event_cell(prs, event, schema, gaps))
        .collect()
}

pub(super) fn event_cell(
    prs: &[&SharedSelection],
    event: &str,
    schema: Schema,
    gaps: &BTreeMap<&str, &str>,
) -> Value {
    if prs.iter().any(|pr| selection_matches_event(pr, event)) {
        return qualified_summary(
            prs.iter()
                .copied()
                .filter(|pr| selection_matches_event(pr, event)),
            schema,
        );
    }
    match gaps.get(event) {
        Some(line) => Value::text(line),
        None => Value::Empty,
    }
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

const SUMMARY_OVERFLOW: &str = "; additional classified marks in PRs";
const SUMMARY_BUDGET: usize = 32_767_usize.saturating_sub(SUMMARY_OVERFLOW.len());

fn qualified_summary<'a>(
    mut prs: impl Iterator<Item = &'a SharedSelection>,
    schema: Schema,
) -> Value {
    let result = prs.try_fold((String::new(), 0_usize, 0_usize), |state, pr| {
        append_summary_mark(state, pr, schema)
    });
    match result {
        Ok((text, _, _)) if text.is_empty() => Value::Empty,
        Ok((text, _, _)) => Value::text(text),
        Err(mut text) => {
            text.push_str(SUMMARY_OVERFLOW);
            Value::text(text)
        }
    }
}

fn append_summary_mark(
    (mut text, used, marker_end): (String, usize, usize),
    pr: &SharedSelection,
    schema: Schema,
) -> Result<(String, usize, usize), String> {
    let mark = labels::qualified_mark(pr, schema);
    let separator = if text.is_empty() { 0 } else { 2 };
    let total = used
        .saturating_add(mark.encode_utf16().count())
        .saturating_add(separator);
    if total > 32_767 {
        text.truncate(marker_end);
        return Err(text);
    }
    if separator != 0 {
        text.push_str("; ");
    }
    text.push_str(&mark);
    let marker_end = if total <= SUMMARY_BUDGET {
        text.len()
    } else {
        marker_end
    };
    Ok((text, total, marker_end))
}
