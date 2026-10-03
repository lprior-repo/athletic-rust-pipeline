use super::{EntityCounts, Stats};
use crate::AdapterReport;
const WORST_FAILURES: usize = 5;

pub(super) fn note_entities(report: &mut AdapterReport, counts: &EntityCounts) {
    report.note(format!(
        "entities: {} meets, {} events, {} teams, {} athletes, {} performances",
        counts.meets, counts.events, counts.teams, counts.athletes, counts.performances
    ));
}

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
