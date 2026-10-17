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
mod retained_view;
pub use retained_view::{
    FrozenRetainedJournal, RecordingRetainedJournal, RetainedJournalBundle, RetainedJournalView,
};
mod roster;
mod wire;

pub use fetch::{
    fetch_owned_meet, fetch_result_set, fetch_roster, fetch_team_index, owned_meet_request,
};
pub(crate) use index::validate_surface as validate_retained_index_surface;
pub use index::{fetch_meet_index, MeetIndexPage, MeetIndexRequest, PageContinuation};
pub(crate) use map::ProviderSchools;
pub use mark::{parse_published_metric_distance, parse_published_time};
pub use normalize::roster_entities;
pub use owned::{
    parse_owned_meet, read_retained_acquisition, read_retained_acquisition_from, CaptureReadBudget,
    CaptureWitnessError, OwnedAcquisitionRef, OwnedCaptureResourceFailure, OwnedCaptureRetention,
    OwnedCohort, OwnedCompleteness, OwnedDocumentFailure, OwnedMeetOutcome, OwnedMeetPage,
    OwnedMeetParseError, OwnedMeetVerdict, OwnedParseArithmetic, OwnedParseLocator,
    OwnedParseResource, OwnedParseResourceFailure, OwnedPerformance, OwnedRejection,
    OwnedRejectionKind, OwnedResourceReceipt, VerifiedOwnedAcquisition, MAX_OWNED_BODY_BYTES,
    OWNED_CAPTURE_PHASE, OWNED_FIELDS, OWNED_MEET_PHASE,
};
pub use owned::{
    read_retained_owned_result, read_retained_owned_result_from, RetainedResultError,
    VerifiedRetainedOwnedResult,
};
pub use parse::{
    has_next_page, parse_captured_roster, parse_meet_index, parse_meet_result_files, parse_roster,
    parse_team_index, TeamIndexRead,
};
pub(crate) use parse::parse_captured_roster_with_witness_v1;
pub(crate) use raw::located::{parse_raw_located_bound, ProvisionalRawRowLocatorV1};
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
    CurrentRosterAssertion, LocatedRosterAthlete, MeetRef, MeetResultFile, ResultSetRef, Roster,
    RosterAthlete, RosterTemporalUnavailable, Season, Site, TeamRef,
    PublishedRosterOwnerKindV1, PublishedRosterOwnerTerminationV1, PublishedRosterOwnersV1,
    PublishedRosterOwnerV1, RosterRowStatementsV1, RosterWitnessUnavailableV1,
};

#[cfg(test)]
mod tests;

pub use results::{
    read_retained_result_metadata, read_retained_result_metadata_from, RawMetadataArchiveRef,
    RetainedMetadataError, RetainedPerformanceDate, RetainedResultDivision, RetainedResultSession,
    VerifiedRetainedResultMetadata,
};
