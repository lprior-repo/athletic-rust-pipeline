
use super::map::Stats;
use super::map_store::EntityCounts;
use crate::AdapterReport;
use census_domain::model::Gender;

pub(super) fn gender_word(gender: Gender) -> &'static str {
    match gender {
        Gender::Girls => "girls",
        Gender::Boys => "boys",
        _ => "unknown",
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Walked {
    pub(super) meets: usize,
    pub(super) skipped_meets: usize,
    pub(super) summaries: usize,
    pub(super) resumed_lists: usize,
}

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
