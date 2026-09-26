//! Published-payload shapes for the IHSA tournament surface.
//!
//! Five payloads, all `Content-Type: application/json` on `https://api.ihsa.org`:
//!
//! * [`MeetsEnvelope`] - `GET /v1/track-field/meets`.
//! * [`EventsEnvelope`] / [`EventRow`] - `GET /v1/track-field/meets/{year}/events?gender=Boys|Girls`.
//! * [`EventSummary`] / [`FinisherRow`] - `GET /v1/track-field/events/{eventId}/summary`.
//! * [`QualifiersEnvelope`] / [`QualifierTeam`] / [`QualifierAthlete`] / [`BoxRow`] -
//!   `GET /v1/{term}/statefinal/cc-qualifiers?tournamentId={id}`.
//! * [`TermsEnvelope`] / [`TermRow`] - `GET /v1/terms`.
//!
//! Only fields this adapter reads are declared; a payload field that is not declared is a field the
//! adapter cannot accidentally read (see the module doc for the declined list). Every id field is
//! `Option`, so a payload that stops publishing one yields a row without that seed rather than a
//! decode failure.
use serde::Deserialize;

/// Envelope returned by `GET /v1/track-field/meets`.
#[derive(Debug, Clone, Deserialize)]
pub struct MeetsEnvelope {
    /// Number of rows the API counted; `data.len()` is the authority.
    pub count: u32,
    pub data: Vec<MeetRow>,
}

/// One state-final meet. `MeetId` equals the Athletic.net Live meet id
/// (`https://live.athletic.net/meets/74003`).
#[derive(Debug, Clone, Deserialize)]
pub struct MeetRow {
    #[serde(rename = "MeetId")]
    pub meet_id: u64,
    #[serde(rename = "Year")]
    pub year: String,
    #[serde(rename = "Title")]
    pub title: String,
    #[serde(rename = "Gender")]
    pub gender: String,
    /// The scoring runner's last refresh; the coarse change signal for re-crawling this meet.
    #[serde(rename = "LastRefreshedAt")]
    pub last_refreshed_at: Option<String>,
}

/// Envelope returned by `GET /v1/track-field/meets/{year}/events?gender=...`.
#[derive(Debug, Clone, Deserialize)]
pub struct EventsEnvelope {
    #[serde(rename = "meetId")]
    pub meet_id: u64,
    pub count: u32,
    pub data: Vec<EventRow>,
}

/// One event instance of a meet: a class x round x event combination.
#[derive(Debug, Clone, Deserialize)]
pub struct EventRow {
    /// Stable per event instance, e.g. `"2790204"`.
    #[serde(rename = "eventId")]
    pub event_id: String,
    /// `"IndividualEvent"` or `"RelayEvent"`.
    #[serde(rename = "eventType")]
    pub event_type: String,
    /// `"M"` or `"F"`.
    pub gender: String,
    /// Class token: `"1A"`, `"2A"`, `"3A"`, `"WD"`.
    #[serde(rename = "classDivision")]
    pub class_division: String,
    /// e.g. `"Boys High Jump 1A - Finals"`.
    #[serde(rename = "eventName")]
    pub event_name: String,
    /// `"P"`, `"F"`, absent for a field final.
    pub round: Option<String>,
    /// `"final"` once the event is complete.
    pub status: Option<String>,
    #[serde(rename = "hasResults")]
    pub has_results: bool,
    /// Event date, `"2026-05-28T00:00:00.000Z"`.
    #[serde(rename = "scheduledDate")]
    pub scheduled_date: Option<String>,
    /// When the scoring runner last pulled this event's metadata.
    #[serde(rename = "metadataFetchedAt")]
    pub metadata_fetched_at: Option<String>,
}

/// `GET /v1/track-field/events/{eventId}/summary`.
#[derive(Debug, Clone, Deserialize)]
pub struct EventSummary {
    #[serde(rename = "eventId")]
    pub event_id: String,
    #[serde(rename = "meetId")]
    pub meet_id: u64,
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub gender: String,
    #[serde(rename = "classDivision")]
    pub class_division: String,
    #[serde(rename = "eventName")]
    pub event_name: String,
    pub round: Option<String>,
    #[serde(rename = "roundLabel")]
    pub round_label: Option<String>,
    #[serde(rename = "scheduledDate")]
    pub scheduled_date: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "hasResults")]
    pub has_results: bool,
    /// The flat result list: one row per placed athlete, or per relay team.
    #[serde(default)]
    pub finishers: Vec<FinisherRow>,
}

/// One placed row of an event summary.
///
/// An individual event fills `athlete`/`athlete_name`/`year`; a relay event fills `members` instead
/// and leaves those three absent.
#[derive(Debug, Clone, Deserialize)]
pub struct FinisherRow {
    pub place: Option<u16>,
    /// Published mark, e.g. `"2.02m"`, `"7:51.37"`; a qualifier suffix (`"1.88mq"`) is stripped by
    /// the parser.
    pub mark: Option<String>,
    #[serde(rename = "athleteName")]
    pub athlete_name: Option<String>,
    /// In-school grade as published, e.g. `"11"`.
    pub year: Option<String>,
    #[serde(rename = "teamName")]
    pub team_name: Option<String>,
    /// The school's own id, the resolution channel for the finisher's school.
    #[serde(rename = "ihsaSchoolId")]
    pub ihsa_school_id: Option<String>,
    pub athlete: Option<AthleteRef>,
    pub team: Option<TeamRef>,
    /// Relay legs; empty for an individual event.
    #[serde(default)]
    pub members: Vec<RelayMember>,
}

/// The identity object the summary publishes for an athlete.
#[derive(Debug, Clone, Deserialize)]
pub struct AthleteRef {
    /// Athletic.net AthleteID, delivered by IHSA for T&F state-final finishers.
    #[serde(rename = "athleticNetId")]
    pub athletic_net_id: Option<u64>,
    /// Athletic.net Live athlete id.
    #[serde(rename = "athleticLiveId")]
    pub athletic_live_id: Option<u64>,
    pub name: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
    /// In-school grade; equals the row's `year` in every captured row.
    pub year: Option<String>,
}

/// The identity object the summary publishes for a team.
#[derive(Debug, Clone, Deserialize)]
pub struct TeamRef {
    #[serde(rename = "athleticNetId")]
    pub athletic_net_id: Option<u64>,
    #[serde(rename = "athleticLiveId")]
    pub athletic_live_id: Option<u64>,
    pub name: Option<String>,
}

/// One relay leg.
#[derive(Debug, Clone, Deserialize)]
pub struct RelayMember {
    pub order: Option<u32>,
    pub athlete: Option<AthleteRef>,
}

/// `GET /v1/{term}/statefinal/cc-qualifiers?tournamentId={id}`.
#[derive(Debug, Clone, Deserialize)]
pub struct QualifiersEnvelope {
    /// Punctuated id as published, e.g. `"688"`; the request carries the same value.
    #[serde(rename = "tournamentId")]
    pub tournament_id: String,
    /// Field-size sector assignments: one row per qualifying athlete, no grade.
    #[serde(rename = "boxAssignments", default)]
    pub box_assignments: Vec<BoxRow>,
    /// Team qualifiers, each with a `teamPlace`.
    #[serde(rename = "teamQualifiers", default)]
    pub team_qualifiers: Vec<QualifierTeam>,
    /// Individual qualifiers, no `teamPlace`.
    #[serde(rename = "individualQualifiers", default)]
    pub individual_qualifiers: Vec<QualifierTeam>,
}

/// One box (sector) assignment row.
#[derive(Debug, Clone, Deserialize)]
pub struct BoxRow {
    #[serde(rename = "Box")]
    pub box_id: Option<String>,
    /// `"T"` for a team-qualifier leg, `"I"` for an individual qualifier.
    #[serde(rename = "Type")]
    pub entry_type: Option<String>,
    #[serde(rename = "SchoolName")]
    pub school_name: Option<String>,
    #[serde(rename = "FirstName")]
    pub first_name: Option<String>,
    #[serde(rename = "LastName")]
    pub last_name: Option<String>,
}

/// One school's qualifiers.
#[derive(Debug, Clone, Deserialize)]
pub struct QualifierTeam {
    #[serde(rename = "schoolName")]
    pub school_name: Option<String>,
    /// The school's own id: the resolution channel for the qualifier's school.
    #[serde(rename = "ihsaSchoolId")]
    pub ihsa_school_id: Option<String>,
    /// Final team place, published only for team qualifiers.
    #[serde(rename = "teamPlace")]
    pub team_place: Option<u32>,
    #[serde(default)]
    pub athletes: Vec<QualifierAthlete>,
}

/// One qualifying athlete.
#[derive(Debug, Clone, Deserialize)]
pub struct QualifierAthlete {
    /// Entry number, unique within the tournament.
    pub number: Option<u32>,
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
    /// In-school grade as published, e.g. `"12"`.
    #[serde(rename = "yearInSchool")]
    pub year_in_school: Option<String>,
    /// Sector assignment, e.g. `"40D"`.
    #[serde(rename = "box")]
    pub r#box: Option<String>,
}

/// `GET /v1/terms`: the terms the archive API answers for.
#[derive(Debug, Clone, Deserialize)]
pub struct TermsEnvelope {
    /// The school year in progress, which has no cross-country archive yet.
    #[serde(rename = "currentTerm")]
    pub current_term: String,
    /// Terms in descending order; the first is the newest completed one.
    #[serde(default)]
    pub terms: Vec<TermRow>,
}

/// One term row.
#[derive(Debug, Clone, Deserialize)]
pub struct TermRow {
    /// e.g. `"2025-26"`.
    pub term: String,
    #[serde(default)]
    pub routes: Option<String>,
}

/// The error envelope the archive API returns for a term it does not hold, e.g.
/// `{"error": "Archive not available for term 2026-27"}`.
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorEnvelope {
    pub error: String,
}
