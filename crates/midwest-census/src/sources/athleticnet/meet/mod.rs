//! Whole-meet pull: the documents a results pull spends, the readers that settle what a published
//! column denotes, and the walk into canonical entities.
//!
//! # Request cost, measured (meet 634313, the anonymous probe of 2026-09-22)
//!
//! A whole meet is a **2-request** pull:
//!
//! 1. `Meet/GetMeetData?meetId=<id>&sport=tf` — the meet row (`meet`), the division id ↔ name table
//!    (`tfDivisions`) and the `jwtMeet` token the results call echoes back as `anettokens`; it also
//!    publishes `eventDivsWithResults` (45 entries for this meet — what the per-event fallback would
//!    cost) and `sport2` (`"tfo"` outdoor, `"tfi"` indoor).
//! 2. `Meet/GetAllResultsData?meetId=<id>&sport=tf&rawResults=false&showTips=false` — every block,
//!    row and relay leg. This meet returned **758 rows in 49 blocks plus 288 relay legs in 72
//!    relays (1,046 rows)** in that one response, and two anonymous re-runs of the pair were
//!    byte-identical (`samples/anon-meet-probe-report.json`).
//!
//! The third document, `Meet/GetEventDivisionData?meetId=<id>&sport=tf`, publishes per-event
//! metadata only (`events[]`: `Type` = `"T"` track / `"F"` field, `isHurdle`, `FieldMeasureType`;
//! 36 entries for this meet) and is spent only under
//! [`Options::event_metadata`](super::Options::event_metadata). It is what settles the mark for an
//! event whose own label maps to no platform kind, and what cross-checks a label against the
//! published type: without it such a row is refused rather than guessed at. (`Meet/GetResultsData3`
//! — one request per event × division × gender, 45 for this meet — is the fallback that adds
//! `Wind`/`Heat`; this adapter does not spend it.)
//!
//! # Rows, as this meet published them
//!
//! 758 block rows = 686 individual + 72 relay squad rows; 63 individual rows carry a no-mark word
//! (`DNS` 39, `NH` 20, `DNF` 2, `DQ` 2, `FS`, `ND`) and 2 squad rows do (a `DNS` and a `DQ` relay),
//! so 623 individual results and 280 of the 288 legs are storable — the 8 legs that are not belong
//! to the two squads whose own mark is a no-mark word. The walk stores 903 performances from 490
//! athletes over 10 schools (20 teams: both gender sides), 49 event identities (4 of the 49 blocks
//! publish a round the other three of their group do not) and one meet.
//!
//! Grades are all published: the 623 stored individual rows carry 9: 87, 10: 146, 11: 197, 12: 193,
//! and the 280 legs 9: 38, 10: 64, 11: 82, 12: 96. The 71 squad rows publishing `"-"` are never
//! read as a grade — a squad's grade is not a person's — and `99`, the placeholder the rankings
//! document publishes on masked rows, is refused by the same reader (`tests.rs`). Rows carry 10
//! distinct schools, each with a team entry whose `IDSchool` resolves the row's `TeamID` (758/758,
//! no name disagreement).
//!
//! # What a row does *not* say
//!
//! The row's `TeamID` is a school id, not a team-season id (it equals the team entry's `IDSchool`),
//! so the meet path stamps no team identity and mints teams exactly as the bio path does. Relay
//! squads are never attributed as individual rows: a squad row's `FirstName` holds the four legs as
//! `<BR>`-joined markup and its `LastName` is null, and the legs arrive separately in `relayLegs`,
//! keyed to their parent by `ResultID` with their own athlete id and grade. A relay leg's
//! performance carries the squad's mark and place — the payload publishes no per-leg split on this
//! meet — and the leg's position is its 1-based order in the parent relay.
//!
//! Nothing here mints a jurisdiction, school, season or grade the payload does not publish: a meet
//! whose `Location.State` is absent or outside the 51 is counted and dropped (the `Overseas` region
//! publishes no state code at all), and a grade is read only from `9`..=`12`.

mod collect;
mod count;
mod map;
mod read;
mod store;
#[cfg(test)]
mod tests;
mod wire;

pub(in crate::sources::athleticnet) use collect::collect as collect_meets;
pub(in crate::sources::athleticnet) use map::absorb_meet;
pub use read::{grade_of, jurisdiction_of, EventMetadata};
pub use wire::{
    AllResults, EventDivisions, FlatEvent, FlatRow, MeetData, PublishedEvent, PublishedLeg,
    PublishedMeet, PublishedTeam,
};

/// `GET /api/v1/Meet/GetMeetData?meetId=<id>&sport=tf` — the first request of a whole-meet pull.
const MEET_ENDPOINT: &str = "https://www.athletic.net/api/v1/Meet/GetMeetData";

/// `GET /api/v1/Meet/GetAllResultsData?…` — the second, results-bearing request.
const RESULTS_ENDPOINT: &str = "https://www.athletic.net/api/v1/Meet/GetAllResultsData";

/// `GET /api/v1/Meet/GetEventDivisionData?…` — the optional third request.
const METADATA_ENDPOINT: &str = "https://www.athletic.net/api/v1/Meet/GetEventDivisionData";

/// The two requests a whole-meet pull spends, in the order it spends them.
pub fn meet_requests(meet_id: i64) -> [String; 2] {
    [
        format!("{MEET_ENDPOINT}?meetId={meet_id}&sport=tf"),
        format!("{RESULTS_ENDPOINT}?meetId={meet_id}&sport=tf&rawResults=false&showTips=false"),
    ]
}

/// The optional third request: per-event metadata only, never results.
pub fn metadata_request(meet_id: i64) -> String {
    format!("{METADATA_ENDPOINT}?meetId={meet_id}&sport=tf")
}
