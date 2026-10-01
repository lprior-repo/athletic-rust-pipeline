mod fetch;
mod map;
mod normalize;
mod parse;
mod raw;
mod raw_issue;
mod raw_rows;
mod results;
mod roster;
mod wire;

pub use fetch::{
    fetch_meet_index, fetch_meet_result_files, fetch_result_set, fetch_roster, fetch_team_index,
};
pub use normalize::roster_entities;
pub use parse::{
    has_next_page, parse_meet_index, parse_meet_result_files, parse_roster, parse_team_index,
};
pub use raw::{parse_raw, RawPage};
pub use raw_issue::{RawGradeIssue, RawGradeIssueKind};
pub use results::{
    collect as collect_result_sets, is_results_page, read_meet_pages, ListedResultFile, MeetPage,
    MeetPages, ResultSetOptions, ResultSetRequest, MISMATCH_LIMIT,
};
pub use roster::{
    RosterOutcome, RosterQuarantine, RosterRejection, RosterRejectionKind, RosterVerdict,
    SourceRowLocator,
};
pub use wire::{
    MeetRef, MeetResultFile, ResultSetRef, Roster, RosterAthlete, Season, Site, TeamRef,
};

#[cfg(test)]
mod tests;
