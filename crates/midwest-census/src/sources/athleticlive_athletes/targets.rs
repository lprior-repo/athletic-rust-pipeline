//! Meet selection: which timer-published meets this adapter queries, keyed by timer meet id.

use census_domain::model::{CanonicalMeet, SourceNamespace};
use census_domain::UsJurisdiction;
use std::collections::{BTreeSet, HashMap};

/// A canonical meet plus the timer identity this adapter queries it by.
#[derive(Debug, Clone)]
pub struct MeetTarget {
    pub athleticlive_meet_id: u64,
    pub meet_id: String,
    pub tenant: String,
    pub name: String,
    pub state: UsJurisdiction,
    pub date: String,
}

/// Meets the adapter will query, plus what it refused to query.
#[derive(Debug, Clone, Default)]
pub struct MeetSelection {
    pub targets: Vec<MeetTarget>,
    /// Timer-published meets dropped because their published date cannot be trusted.
    ///
    /// Tenant meet indexes carry placeholder rows (`date` in the 2220s). An athlete's graduating
    /// class is derived from the meet date, so an implausible date would mint an implausible
    /// class; those meets are skipped rather than guessed at.
    pub skipped_implausible: usize,
    /// Meets dropped because no evidence placed them in a jurisdiction.
    ///
    /// A meet row carries its jurisdiction into every school it mints, so a meet whose venue was
    /// never placed cannot produce a school without inventing one; the count is reported instead of
    /// the rows being filed under a state nobody observed.
    pub skipped_unplaced: usize,
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
pub fn meet_targets(meets: &[CanonicalMeet], states: &[UsJurisdiction]) -> MeetSelection {
    let wanted: BTreeSet<UsJurisdiction> = states.iter().copied().collect();
    let mut seen: HashMap<u64, MeetTarget> = HashMap::new();
    let mut skipped_implausible = 0usize;
    let mut skipped_unplaced = 0usize;
    for meet in meets {
        // The jurisdiction reaches every school the meet's rows mint, so a meet that has none is
        // not queryable: its count is reported rather than its rows being filed under a guess.
        let Some(state) = meet.state else {
            skipped_unplaced = skipped_unplaced.saturating_add(1);
            continue;
        };
        if !wanted.is_empty() && !wanted.contains(&state) {
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
                    state,
                    date: meet.date.clone(),
                });
        }
    }
    let mut targets: Vec<MeetTarget> = seen.into_values().collect();
    targets.sort_by_key(|target| {
        (
            target.state,
            target.date.clone(),
            target.athleticlive_meet_id,
        )
    });
    MeetSelection {
        targets,
        skipped_implausible,
        skipped_unplaced,
    }
}
