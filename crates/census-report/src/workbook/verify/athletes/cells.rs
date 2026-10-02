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
    qualified_summary(prs.iter().copied())
}

pub(super) fn pr_events(prs: &[&SharedSelection]) -> Vec<Value> {
    labels::PR_EVENTS
        .iter()
        .map(|event| event_cell(prs, event))
        .collect()
}

pub(super) fn event_cell(prs: &[&SharedSelection], event: &str) -> Value {
    qualified_summary(
        prs.iter()
            .copied()
            .filter(|pr| pr.key.event_kind.stable_key() == event),
    )
}

fn qualified_summary<'a>(mut prs: impl Iterator<Item = &'a SharedSelection>) -> Value {
    const OVERFLOW: &str = "; additional classified marks in PRs";
    let budget = 32_767_usize.saturating_sub(OVERFLOW.len());
    let result = prs.try_fold(
        (String::new(), 0_usize, 0_usize),
        |(mut text, used, marker_end), pr| {
            let mark = labels::qualified_mark(pr);
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
            let marker_end = if total <= budget {
                text.len()
            } else {
                marker_end
            };
            Ok((text, total, marker_end))
        },
    );
    match result {
        Ok((text, _, _)) if text.is_empty() => Value::Empty,
        Ok((text, _, _)) => Value::text(text),
        Err(mut text) => {
            text.push_str(OVERFLOW);
            Value::text(text)
        }
    }
}
