use std::collections::BTreeSet;

use crate::bests::SharedSelection;
use census_domain::model::{CanonicalAthlete, ObservedGrade};

use super::super::canonical::Value;
use super::super::labels;

pub(super) fn newest_observation(athlete: &CanonicalAthlete) -> Option<&ObservedGrade> {
    athlete
        .observed_grades
        .iter()
        .max_by_key(|grade| (grade.school_year.get(), grade.grade.get()))
}

pub(super) fn flag(recorded: bool) -> Value {
    Value::flagged(recorded)
}

pub(super) fn source_count(athlete: &CanonicalAthlete) -> usize {
    athlete
        .identities()
        .map(|identity| &identity.namespace)
        .collect::<BTreeSet<_>>()
        .len()
}

#[derive(Default)]
pub(super) struct Profiles {
    pub(super) athletic_net: Option<String>,
    pub(super) milesplit: Option<String>,
    pub(super) other: Vec<String>,
}

pub(super) fn profiles_of(athlete: &CanonicalAthlete) -> Profiles {
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
        let lowered = url.to_ascii_lowercase();
        if lowered.contains("athletic.net") {
            if profiles.athletic_net.is_none() {
                profiles.athletic_net = Some(url);
            }
        } else if lowered.contains("milesplit") {
            if profiles.milesplit.is_none() {
                profiles.milesplit = Some(url);
            }
        } else {
            profiles.other.push(url);
        }
    }
    profiles
}

pub(super) fn event_list(prs: &[&SharedSelection]) -> Value {
    if prs.is_empty() {
        return Value::Empty;
    }
    let events: BTreeSet<_> = prs
        .iter()
        .map(|pr| pr.key.event_kind.stable_key())
        .collect();
    let names: Vec<&str> = events
        .iter()
        .map(|event| labels::pr_event_name(event))
        .collect();
    Value::text(names.join("; "))
}

pub(super) fn headline(prs: &[&SharedSelection]) -> Value {
    if prs.is_empty() {
        return Value::Empty;
    }
    let mut text = String::new();
    for pr in prs.iter().take(10) {
        append_mark(&mut text, pr);
    }
    if prs.len() > 10 {
        text.push_str("; additional classified marks in PRs");
    }
    Value::text(text)
}

pub(super) fn pr_events(prs: &[&SharedSelection]) -> Vec<Value> {
    labels::PR_EVENTS
        .iter()
        .map(|event| event_cell(prs, event))
        .collect()
}

pub(super) fn event_cell(prs: &[&SharedSelection], event: &str) -> Value {
    let mut selected = prs
        .iter()
        .copied()
        .filter(|pr| pr.key.event_kind.stable_key() == event);
    let Some(first) = selected.next() else {
        return Value::Empty;
    };
    let mut text = labels::qualified_mark(first);
    for pr in selected {
        text.push_str("; ");
        text.push_str(&labels::qualified_mark(pr));
    }
    Value::text(text)
}

fn append_mark(text: &mut String, pr: &SharedSelection) {
    if !text.is_empty() {
        text.push_str("; ");
    }
    text.push_str(&labels::qualified_mark(pr));
}
