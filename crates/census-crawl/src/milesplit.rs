pub mod boundary;
mod fetch;
mod index;
mod map;
mod mark;
mod normalize;
mod owned;
mod parse;
mod raw;
mod raw_issue;
mod raw_rows;
mod results;
mod roster;
mod wire;

pub use fetch::{
    fetch_owned_meet, fetch_result_set, fetch_roster, fetch_team_index, owned_meet_request,
};
pub use index::{fetch_meet_index, MeetIndexPage, MeetIndexRequest, PageContinuation};
pub use mark::{parse_published_metric_distance, parse_published_time};
pub use normalize::roster_entities;
pub use owned::{
    parse_owned_meet, read_retained_acquisition, CaptureReadBudget, CaptureWitnessError,
    OwnedAcquisitionRef, OwnedCaptureResourceFailure, OwnedCaptureRetention, OwnedCohort,
    OwnedCompleteness, OwnedDocumentFailure, OwnedMeetOutcome, OwnedMeetPage, OwnedMeetParseError,
    OwnedMeetVerdict, OwnedParseArithmetic, OwnedParseLocator, OwnedParseResource,
    OwnedParseResourceFailure, OwnedPerformance, OwnedRejection, OwnedRejectionKind,
    OwnedResourceReceipt, VerifiedOwnedAcquisition, MAX_OWNED_BODY_BYTES, OWNED_CAPTURE_PHASE,
    OWNED_FIELDS, OWNED_MEET_PHASE,
};
pub use owned::{read_retained_owned_result, RetainedResultError, VerifiedRetainedOwnedResult};
pub use parse::{
    has_next_page, parse_captured_roster, parse_meet_index, parse_meet_result_files, parse_roster,
    parse_team_index, TeamIndexRead,
};
pub use raw::{parse_raw, RawPage};
pub use raw_issue::{RawGradeIssue, RawGradeIssueKind};
pub use results::{
    collect as collect_result_sets, is_results_page, read_meet_pages, BoundedMeetPages,
    ListedResultFile, MeetPage, MeetPageLimits, MeetPages, ResultSetOptions, ResultSetRequest,
    MISMATCH_LIMIT, RESULT_SET_PHASE,
};
pub use roster::{
    RosterOutcome, RosterQuarantine, RosterRejection, RosterRejectionKind, RosterVerdict,
    SourceRowLocator,
};
pub use wire::{
    LocatedRosterAthlete, MeetRef, MeetResultFile, ResultSetRef, Roster, RosterAthlete, Season,
    Site, TeamRef,
};

#[cfg(test)]
mod tests;

pub use results::{
    read_retained_result_metadata, RawMetadataArchiveRef, RetainedMetadataError,
    RetainedPerformanceDate, RetainedResultDivision, VerifiedRetainedResultMetadata,
};
