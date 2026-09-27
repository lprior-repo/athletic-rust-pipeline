
use super::map::Stats;
use super::run::Run;
use crate::AdapterReport;

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct EntityCounts {
    pub(super) schools: usize,
    pub(super) meets: usize,
    pub(super) teams: usize,
    pub(super) athletes: usize,
    pub(super) events: usize,
    pub(super) performances: usize,
}

const WORST_FAILURES: usize = 20;

pub(super) fn note_pages(report: &mut AdapterReport, run: &Run<'_>) {
    let stats = &run.absorb.stats;
    report.note(format!(
        "pages: {} read, {} already journaled at this phase, {} list sections, {} rosters",
        run.pages, run.resumed, stats.sections, stats.rosters_seen
    ));
}

pub(super) fn note_rows(report: &mut AdapterReport, stats: &Stats) {
    report.note(format!(
        "performance-list rows: {} seen, {} absorbed, {} relay rows ({} members; a relay row prints \
         surnames only, so no athlete is minted from it)",
        stats.rows_seen,
        stats.rows_absorbed,
        stats.rows_relay,
        stats.relay_members
    ));
    report.note(format!(
        "skipped rows: {} without a team, {} without a season (the route states none), {} without an \
         athlete, {} without a mark the reader can place, {} without a meet, {} without a date",
        stats.rows_without_team,
        stats.rows_without_season,
        stats.rows_without_athlete,
        stats.rows_without_mark,
        stats.rows_without_meet,
        stats.rows_without_date
    ));
    report.note(format!(
        "marks: {} carrying the host's own conversion, {} kept in the notation the row published, \
         {} sections whose event mapped to no kind",
        stats.marks_converted, stats.marks_unconverted, stats.events_unmapped
    ));
    report.note(format!(
        "grades: {} rows placed by the page's `year=` filter (they print no year), {} rows whose \
         filter disagrees with the year they print (the printed year wins), {} rows below high \
         school, {} rows whose gender the page does not state",
        stats.grades_from_filter,
        stats.grades_conflicting_filter,
        stats.rows_below_high_school,
        stats.genders_unknown
    ));
}

pub(super) fn note_rosters(report: &mut AdapterReport, stats: &Stats) {
    report.note(format!(
        "rosters: {} rows seen, {} absorbed, {} without a name, {} pages whose season control states \
         no season or no sport (their rows are skipped)",
        stats.roster_rows_seen,
        stats.roster_rows_absorbed,
        stats.roster_rows_without_name,
        stats.rosters_without_season
    ));
}

pub(super) fn note_resolution(report: &mut AdapterReport, stats: &Stats) {
    report.note(format!(
        "schools: {} resolved against the consolidated index, {} minted from this source",
        stats.schools_resolved, stats.schools_minted
    ));
}

pub(super) fn note_entities(report: &mut AdapterReport, counts: &EntityCounts) {
    report.note(format!(
        "canonical entities: schools {} meets {} teams {} athletes {} events {} performances {}",
        counts.schools,
        counts.meets,
        counts.teams,
        counts.athletes,
        counts.events,
        counts.performances
    ));
}

pub(super) fn note_failures(report: &mut AdapterReport, failures: &[String]) {
    for failure in failures.iter().take(WORST_FAILURES) {
        report.note(format!("failure: {failure}"));
    }
    if failures.len() > WORST_FAILURES {
        report.note(format!(
            "… and {} more failure(s)",
            failures.len().saturating_sub(WORST_FAILURES)
        ));
    }
}
