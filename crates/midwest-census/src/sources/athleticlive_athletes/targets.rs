//! Meet selection: which timer-published meets this adapter queries, keyed by timer meet id.

use census_domain::model::{CanonicalMeet, SourceNamespace};
use std::collections::{BTreeSet, HashMap};

/// A canonical meet plus the timer identity this adapter queries it by.
#[derive(Debug, Clone)]
pub struct MeetTarget {
    pub athleticlive_meet_id: u64,
    pub meet_id: String,
    pub tenant: String,
    pub name: String,
    pub state: String,
    pub date: String,
}

/// Meets the adapter will query, plus the count it refused to query.
#[derive(Debug, Clone, Default)]
pub struct MeetSelection {
    pub targets: Vec<MeetTarget>,
    /// Timer-published meets dropped because their published date cannot be trusted.
    ///
    /// Tenant meet indexes carry placeholder rows (`date` in the 2220s). An athlete's graduating
    /// class is derived from the meet date, so an implausible date would mint an implausible
    /// class; those meets are skipped rather than guessed at.
    pub skipped_implausible: usize,
}

/// The window a meet date must fall in to be usable. Same convention as the meet-index harvest.
const MEET_YEAR_MIN: i16 = 2015;
const MEET_YEAR_MAX: i16 = 2030;

fn plausible_meet_year(date: &str) -> bool {
    match date.get(..4).and_then(|year| year.parse::<i16>().ok()) {
        Some(year) => (MEET_YEAR_MIN..=MEET_YEAR_MAX).contains(&year),
        None => false,
    }
}

/// Select meets that a timer published, keyed by their AthleticLIVE meet id.
///
/// Meets are deduplicated by canonical id: several tenants publishing one meet collapse to the first
/// target, because the athlete rows are keyed by the AthleticLIVE meet id and duplicate ids would
/// multiply requests.
pub fn meet_targets(meets: &[CanonicalMeet], states: &[String]) -> MeetSelection {
    let wanted: BTreeSet<String> = states
        .iter()
        .map(|state| state.trim().to_ascii_uppercase())
        .collect();
    let mut seen: HashMap<u64, MeetTarget> = HashMap::new();
    let mut skipped_implausible = 0usize;
    for meet in meets {
        if !wanted.is_empty() && !wanted.contains(&meet.state.to_ascii_uppercase()) {
            continue;
        }
        if !plausible_meet_year(&meet.date) {
            skipped_implausible = skipped_implausible.saturating_add(1);
            continue;
        }
        for identity in &meet.source_identities {
            let SourceNamespace::TimerMeet { provider } = &identity.namespace else {
                continue;
            };
            let Ok(athleticlive_meet_id) = identity.id.parse::<u64>() else {
                continue;
            };
            seen.entry(athleticlive_meet_id)
                .or_insert_with(|| MeetTarget {
                    athleticlive_meet_id,
                    meet_id: meet.id.as_str().to_string(),
                    tenant: provider.clone(),
                    name: meet.name.clone(),
                    state: meet.state.clone(),
                    date: meet.date.clone(),
                });
        }
    }
    let mut targets: Vec<MeetTarget> = seen.into_values().collect();
    targets.sort_by_key(|target| {
        (
            target.state.clone(),
            target.date.clone(),
            target.athleticlive_meet_id,
        )
    });
    MeetSelection {
        targets,
        skipped_implausible,
    }
}
