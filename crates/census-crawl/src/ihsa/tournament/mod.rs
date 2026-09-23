//! IHSA tournament results surface (Illinois): state-final track & field results and the
//! cross-country state-finalist lists, decoded from the captures recorded in
//! `research/midwest/13-illinois-ihsa.md` and `research/sources/state-assoc-greatlakes/`.
//!
//! This adapter is the only Great Lakes association that publishes Athletic.net identity on its own
//! result rows: every T&F state-final finisher (and every relay leg) carries
//! `finishers[].athlete.athleticNetId` and `finishers[].athlete.athleticLiveId`, and every finisher
//! row carries the school's own id in `finishers[].ihsaSchoolId`. Those ids are emitted as evidence
//! rows, never as a value the payload does not carry.
//!
//! # Measured request cost per season
//!
//! * `GET /v1/track-field/meets` - 1 request, `count: 2`: the 2026 boys (meet `74003`) and girls
//!   (`74002`) state finals, each row carrying `LastRefreshedAt` (coarse change signal) and
//!   `LastRefreshSummary.eventsDiscovered` (97 for the boys meet).
//! * `GET /v1/track-field/meets/{year}/events?gender=Boys` - 1 request per meet; the captured boys
//!   index is 97 event rows (200,022 B), every row `status: "final"` with `hasResults: true`.
//! * `GET /v1/track-field/events/{eventId}/summary` - 1 request per event row with results: the
//!   captured finals are 110,259 B (20 individual finishers) and 148,085 B (12 relay teams, 48
//!   legs). A two-gender championship week is therefore ~200 requests (2 index + 2 events + ~194
//!   summaries), the ranking's measured unit.
//! * `GET /v1/terms` - 1 request per run: `terms[0].term` ("2025-26") is the newest term with a
//!   cross-country archive; `currentTerm` ("2026-27") has none (measured: `{"error": "Archive not
//!   available for term 2026-27"}`).
//! * `GET /v1/{term}/statefinal/cc-qualifiers?tournamentId={688..693}` - 6 requests per season
//!   (3 classes x 2 genders). Measured payloads: 88 KB (boys 1A, 1,214 boys across three classes,
//!   358 of them grade 11), 80 KB (girls 1A).
//!
//! # Exact id-bearing field names
//!
//! | id | field | measured value |
//! |---|---|---|
//! | Athletic.net meet id | `MeetId` (meets index) / `meetId` (events index and summary) | `74003` boys, `74002` girls |
//! | Athletic.net athlete id | `finishers[].athlete.athleticNetId` | `27740691` |
//! | Athletic.net Live athlete id | `finishers[].athlete.athleticLiveId` | `49752378` |
//! | Athletic.net team id | `finishers[].team.athleticNetId` | `16352` |
//! | Athletic.net Live team id | `finishers[].team.athleticLiveId` | `1679604` |
//! | relay leg ids | `finishers[].members[].athlete.athleticNetId` / `...athleticLiveId` | 48/48 legs |
//! | school id | `finishers[].ihsaSchoolId` (summary) and `{team,individual}Qualifiers[].ihsaSchoolId` (XC) | `"0611"`, `"0247"` |
//! | event id | `eventId` | `"2790204"` (hiatus-free string, class+round+event) |
//!
//! Cohort fields: `finishers[].year` ("11") and `finishers[].members[].athlete.year` for T&F;
//! `athletes[].yearInSchool` for the XC lists. Both are in-school grades, so they map to
//! `census_domain::model::Grade` (scale 9..=12); a value outside that scale is dropped, not guessed.
//!
//! # Deliberately ignored fields (never read, never stored)
//!
//! `heats[].entries` (the same finishers again, one heat per payload), `relatedRounds` (prelim rows
//! reachable from a final; the index lists those rounds as their own `eventId`), `header`, `records`,
//! `rankings` (Athletic.net list URLs: `https://www.athletic.net/TrackAndField/rankings/list/...`,
//! which belong to the `athleticnet` adapter's namespace, not this one), `metadata` (the scoring
//! engine's internal document), `seedMark`, `previousBest`, `newPersonalBest`, `recordNotation`,
//! `points`, `isValid`, `imperialMark`, `wind`, `lane`, `heat`, `relayDivision`, `placesScored`,
//! `scheduledTs`, `roundLabel` (derived from `round` instead so the index and summary paths agree),
//! and the XC payload's `coaches` names (the staff pass covers coach identity with `PersonID`,
//! role codes and the address reveal; these rows add no id).
//!
//! # Status
//!
//! Implemented here, and exercised against the captures in `tests/fixtures/ihsa_tournament/`:
//!
//! * the wire shapes ([`wire`]) for all five payloads;
//! * the pure decoders ([`parse`]), fixture-pinned in the module's own tests;
//! * the walk ([`collect`]): the meets index, one summary per event with results, then `/v1/terms`
//!   and the six `cc-qualifiers` lists;
//! * the canonical mapping (the private `map` module and its row modules): a finisher's
//!   `athleticNetId` and `athleticLiveId` become `SourceIdentity` rows under the `AthleticNet`
//!   namespace (the same id space the `athleticnet` adapter reads), the team's ids become the
//!   team's, `ihsaSchoolId` resolves the school through the association identities the `ihsa`
//!   schools adapter minted, and `finishers[].year` / `athletes[].yearInSchool` become the
//!   athlete's observed grade and the performance's `observed_grade`.
//!
//! What a run refuses to invent: a finisher row with no published mark stores no performance; a row
//! with no published grade mints no athlete (the graduating class is part of the athlete's natural
//! key); a relay row's own mark is the team's, so its four legs are minted as athletes and no
//! performance claims an individual mark the payload never stated. Every such row is counted in the
//! report's notes.

mod collect;
mod entities;
mod journal;
mod map;
pub mod parse;
mod report;
mod requests;
mod schools;
mod tracks;
pub mod wire;
mod xc;

pub use collect::collect;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod walk_tests;
