//! What one run says about itself: the notes a reader can check against the payloads.
//!
//! Every count here is a count of rows the run actually saw, and every bucket that is not a stored
//! row is named: a finisher without a mark, a row without a grade, a relay leg with no performance of
//! its own, a qualifier row the archive publishes no grade for, a school minted rather than resolved.
//! A run that reads the whole 2026 season should report the measured order of magnitude — two meets,
//! ~194 events, ~200 requests — or say where it stopped.

use super::map::Stats;
use super::map_store::EntityCounts;
use crate::AdapterReport;
use census_domain::model::Gender;

/// The gender word the report spells the archive's lists with.
pub(super) fn gender_word(gender: Gender) -> &'static str {
    match gender {
        Gender::Girls => "girls",
        Gender::Boys => "boys",
        _ => "unknown",
    }
}

/// What one run walked, per route: the meets it read, the meets its journal skipped, the summaries
/// those meets cost, and the qualifier lists the journal already had.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Walked {
    pub(super) meets: usize,
    pub(super) skipped_meets: usize,
    pub(super) summaries: usize,
    pub(super) resumed_lists: usize,
}

/// Report the walk in the numbers a reader can check against the payloads.
pub(super) fn narrate(
    report: &mut AdapterReport,
    stats: &Stats,
    counts: &EntityCounts,
    walked: Walked,
) {
    report.note(format!(
        "track & field: {} meets walked, {} skipped at their published LastRefreshedAt, {} events minted ({} without results), {} event summaries read",
        walked.meets, walked.skipped_meets, stats.events, stats.events_without_results, walked.summaries
    ));
    report.note(format!(
        "finishers: {} rows, {} without a mark, {} without a grade, {} without a resolvable school, {} relay legs minted with no performance row of their own",
        stats.rows, stats.rows_no_mark, stats.rows_no_grade, stats.rows_no_school, stats.legs
    ));
    report.note(format!(
        "cross country: {} qualifier rows, {} of them with a published grade, {} without (an athlete keys on its graduating class, so those mint nothing), {} lists already read",
        stats.qualifier_rows,
        stats.qualifier_graded,
        stats.qualifier_no_grade,
        walked.resumed_lists
    ));
    report.note(format!(
        "canonical: {} schools ({} minted here), {} meets, {} events, {} teams, {} athletes, {} performances",
        counts.schools,
        stats.schools_minted,
        counts.meets,
        counts.events,
        counts.teams,
        counts.athletes,
        counts.performances
    ));
}
