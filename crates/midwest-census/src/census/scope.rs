//! Scope filtering: which units a resumed run still owes, and which athletes the cohort counts.
//!
//! `pending_rosters` is the resume ledger — a roster already journaled is skipped, and the skip is
//! counted rather than assumed away — and the two counters are the class-of-2027 cohort the census
//! is judged on. Both are predicates over a roster, never over a report.

use crate::sources::milesplit::{Roster, TeamRef};
use crate::store::{Store, StoreResult};
use census_domain::model::{Gender, GradYear};

use super::rosters_phase;

/// Rosters not yet journaled for `state`, plus how many were skipped because they already were.
///
/// The filter walks the state's team index once; `journal_keys` is the resume ledger.
pub(super) fn pending_rosters(
    store: &Store,
    teams: &[TeamRef],
    state: &str,
) -> StoreResult<(Vec<TeamRef>, usize)> {
    let done = store.journal_keys(&rosters_phase(state))?;
    let pending: Vec<TeamRef> = teams
        .iter()
        .filter(|team| !done.contains(&format!("{}:{}", state, team.id)))
        .cloned()
        .collect();
    let skipped = teams.len().saturating_sub(pending.len());
    Ok((pending, skipped))
}

/// Class-of-2027 athletes in a roster.
pub(super) fn count_co2027(roster: &Roster) -> usize {
    roster
        .athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027)
        .count()
}

/// Class-of-2027 athletes of one gender in a roster.
pub(super) fn count_cohort(roster: &Roster, gender: Gender) -> usize {
    roster
        .athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027 && athlete.gender == gender)
        .count()
}
