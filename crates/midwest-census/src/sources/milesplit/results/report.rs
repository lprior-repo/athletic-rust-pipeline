//! The notes a result-set run publishes: what it read, what each guard dropped, how the row labels
//! resolved, and what it could not finish.
//!
//! Split out of [`super`] to keep both files inside the source-length budget. Every helper here
//! writes into the run's [`AdapterReport`] and reads nothing but the run counters, so the report is
//! reconstructible from the counters alone.

use super::{EntityCounts, Stats};
use crate::sources::AdapterReport;
use std::collections::HashMap;

/// How many unresolved labels the report names.
const WORST_LABELS: usize = 10;
/// How many failures the report names.
const WORST_FAILURES: usize = 5;

/// Note what the route read and how much it resumed, and the region cross-check it ran.
pub(super) fn note_result_sets(report: &mut AdapterReport, stats: &Stats) {
    report.note(format!(
        "result sets: {} read (1 request each), {} already journaled, {} empty, {} failed",
        stats.result_sets,
        stats.result_sets_resumed,
        stats.result_sets_empty,
        stats.failures.len()
    ));
    if stats.region_mismatch > 0 {
        report.note(format!(
            "{} result set(s) published a region that is not the site's own",
            stats.region_mismatch
        ));
    }
}

/// Note the row-level tally: what the reader kept and what each guard dropped.
pub(super) fn note_rows(report: &mut AdapterReport, stats: &Stats) {
    report.note(format!(
        "rows: {} read, {} with Yr, {} without Yr (no grad-year projection), {} without a school \
         label, {} named after their own school, {} rejected by the name guard, {} without a sport",
        stats.rows,
        stats.rows_with_grade,
        stats.rows_without_grade,
        stats.rows_without_school,
        stats.rows_school_named,
        stats.rows_without_name,
        stats.rows_without_sport
    ));
    report.note(format!(
        "fixed-width lines dropped by the column map: {}",
        stats.skipped_lines
    ));
}

/// Note how the row labels resolved against the consolidated school snapshot.
pub(super) fn note_resolution(report: &mut AdapterReport, stats: &Stats) {
    report.note(format!(
        "school labels resolved: {:?}",
        stats.school_resolved
    ));
    if !stats.unresolved.is_empty() {
        report.note(format!(
            "unresolved school labels (most frequent first): {:?}",
            worst(&stats.unresolved)
        ));
    }
}

/// Note the canonical entities the run wrote.
pub(super) fn note_entities(report: &mut AdapterReport, counts: &EntityCounts) {
    report.note(format!(
        "entities: {} meets, {} events, {} teams, {} athletes, {} performances",
        counts.meets, counts.events, counts.teams, counts.athletes, counts.performances
    ));
}

/// Note the failures, capped so one broken batch cannot bury the summary.
pub(super) fn note_failures(report: &mut AdapterReport, stats: &Stats) {
    for failure in stats.failures.iter().take(WORST_FAILURES) {
        report.note(format!("failure: {failure}"));
    }
    if stats.failures.len() > WORST_FAILURES {
        report.note(format!(
            "… and {} more failure(s)",
            stats.failures.len().saturating_sub(WORST_FAILURES)
        ));
    }
}

/// The most frequent unresolved labels, most frequent first and ties in label order.
fn worst(unresolved: &HashMap<String, usize>) -> Vec<(&str, usize)> {
    let mut rows: Vec<(&str, usize)> = unresolved
        .iter()
        .map(|(label, count)| (label.as_str(), *count))
        .collect();
    rows.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    rows.truncate(WORST_LABELS);
    rows
}
