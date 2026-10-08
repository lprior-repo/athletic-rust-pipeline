use super::{EntityCounts, Stats, ADAPTER};
use crate::{AdapterReport, CrawlError, CrawlResult, UnresolvedCounters};
const WORST_FAILURES: usize = 5;

pub(super) fn finish(
    stats: &Stats,
    counts: &EntityCounts,
    before: (u64, u64),
    after: (u64, u64),
) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new(ADAPTER, "result rows");
    report.rows = counter(stats.rows)?;
    report.errors = counter(stats.failure_count)?;
    report.unresolved = Some(UnresolvedCounters {
        rows: counter(stats.rows_without_school)?,
        labels: counter(stats.unresolved.len())?,
    });
    report.requests = after.0.saturating_sub(before.0);
    report.from_cache = after.1.saturating_sub(before.1);
    note_source(&mut report, stats);
    note_entities(&mut report, counts);
    note_failures(&mut report, stats);
    report.note(format!("result buffers: peak admitted window bytes {}; peak active capture bytes {}; one active meet; no retained whole-source entities", stats.peak_window_bytes, stats.peak_capture_bytes));
    report.note(format!(
        "supplied result locators left unread/unfinished after resource admission: {}",
        stats.unread
    ));
    if let super::ResourceStop::Unfinished(locator) = &stats.resource_stop {
        report.note(format!("resource-exhausted unfinished locator: {locator}"));
    }
    Ok(report)
}

fn counter(value: usize) -> CrawlResult<u64> {
    u64::try_from(value).map_err(|_| CrawlError::Arithmetic {
        detail: "result report count does not fit in u64".into(),
    })
}

fn note_source(report: &mut AdapterReport, stats: &Stats) {
    report.note(format!(
        "owned result sets: {} projected, {} already journaled, {} empty; {} retained failures",
        stats.result_sets, stats.result_sets_resumed, stats.result_sets_empty, stats.failure_count,
    ));
    report.note(format!(
        "owned individual observations: {}; published cohorts: {}; missing/invalid cohorts retained: {}; unresolved provider teamIDs: {}; absent raw sport metadata: {}",
        stats.rows, stats.rows_with_cohort, stats.rows_without_cohort,
        stats.rows_without_school, stats.rows_without_sport,
    ));
    report.note(format!(
        "exact unique MilesplitSchool provider bindings: {:?}; unresolved mappings: {:?}",
        stats.school_resolved, stats.unresolved,
    ));
    report.note("Source completeness remains capture-scoped and may be Unknown; projected observations do not assert canonical identity acceptance, a lifetime PR, source exhaustion or a census seal.");
}

fn note_entities(report: &mut AdapterReport, counts: &EntityCounts) {
    report.note(format!(
        "entities: {} meets, {} events, {} teams, {} athletes, {} performances",
        counts.meets, counts.events, counts.teams, counts.athletes, counts.performances,
    ));
    report.note(format!(
        "unsupported cohort observations retained: {}",
        counts.unsupported_cohorts
    ));
}

fn note_failures(report: &mut AdapterReport, stats: &Stats) {
    stats
        .failures
        .iter()
        .take(WORST_FAILURES)
        .for_each(|failure| report.note(format!("failure: {failure}")));
    if stats.failure_count > WORST_FAILURES {
        report.note(format!(
            "… and {} more failure(s)",
            stats.failure_count.saturating_sub(WORST_FAILURES)
        ));
    }
}

impl Stats {
    pub(super) fn failure(&mut self, reason: String) -> CrawlResult<()> {
        self.failure_count = self.failure_count.saturating_add(1);
        if self.failures.len() < WORST_FAILURES {
            self.failures
                .try_reserve(1)
                .map_err(super::budget::reserve)?;
            let end = reason
                .char_indices()
                .nth(4096)
                .map_or(reason.len(), |(index, _)| index);
            let mut sample = String::new();
            sample
                .try_reserve_exact(end)
                .map_err(super::budget::reserve)?;
            sample.push_str(reason.get(..end).ok_or_else(|| CrawlError::Invariant {
                detail: "failure sample has an invalid text boundary".into(),
            })?);
            self.failures.push(sample);
        }
        Ok(())
    }
}
