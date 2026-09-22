//! The wire format of the two (or three) whole-meet documents, as published.

use super::super::parse::optional_text;
use serde::Deserialize;

/// The meet document: the meet row, its divisions, and the token the results call echoes.
#[derive(Debug, Clone, Deserialize)]
pub struct MeetData {
    #[serde(rename = "meet")]
    pub meet: PublishedMeet,
    #[serde(rename = "tfDivisions", default)]
    pub divisions: Vec<PublishedDivision>,
    /// `jwtMeet` — sent back as the `anettokens` header on the two follow-up requests.
    #[serde(rename = "jwtMeet", default)]
    pub token: Option<String>,
    /// `"tfo"` for outdoor track, `"tfi"` indoor (no indoor capture: unverified).
    #[serde(rename = "sport2", default)]
    pub sport2: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishedMeet {
    #[serde(rename = "ID")]
    pub id: i64,
    #[serde(rename = "Name", default)]
    pub name: String,
    /// `"2026-05-15T00:00:00"`.
    #[serde(rename = "MeetDate", default)]
    pub date: String,
    #[serde(rename = "EndDate", default)]
    pub end_date: Option<String>,
    /// The season the meet belongs to (`2026`), not a school year.
    #[serde(rename = "SeasonID", default)]
    pub season_id: Option<i16>,
    #[serde(rename = "Location", default)]
    pub location: Option<PublishedLocation>,
    /// The AthleticLIVE join key (`73767`); the id space it belongs to there is unverified.
    #[serde(rename = "LiveID", default)]
    pub live_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishedLocation {
    #[serde(rename = "Name", default)]
    pub name: String,
    /// `"WI"` — where the payload places the venue; absent on the out-of-scope `Overseas` region.
    #[serde(rename = "State", default)]
    pub state: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishedDivision {
    #[serde(rename = "IDDiv")]
    pub id: i64,
    #[serde(rename = "Division", default)]
    pub name: String,
}

/// The results document.
#[derive(Debug, Clone, Deserialize)]
pub struct AllResults {
    /// Every block: one event × division × gender × round, with its rows.
    #[serde(rename = "flatEvents", default)]
    pub blocks: Vec<FlatEvent>,
    /// Every relay leg of the meet, keyed to its parent result by `ResultID`.
    #[serde(rename = "relayLegs", default)]
    pub legs: Vec<PublishedLeg>,
    #[serde(rename = "teams", default)]
    pub teams: Vec<PublishedTeam>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlatEvent {
    /// The event id the metadata document carries.
    #[serde(rename = "EventId")]
    pub event_id: i64,
    /// `"100 Meters"`.
    #[serde(rename = "Event", default)]
    pub label: String,
    /// `"100m"` — the label the ontology is keyed on.
    #[serde(rename = "EventShort", default)]
    pub short: String,
    /// `"M"` / `"F"`.
    #[serde(rename = "Gender", default)]
    pub gender: String,
    #[serde(rename = "DivId", default)]
    pub division_id: Option<i64>,
    /// `"Varsity"`; [`PublishedDivision`] is the fallback when a block omits it.
    #[serde(rename = "Division", default)]
    pub division: Option<String>,
    /// `"F"` final, `"P"` prelim.
    #[serde(rename = "Round", default)]
    pub round: Option<String>,
    #[serde(default)]
    pub results: Vec<FlatRow>,
}

impl FlatEvent {
    /// The label canonical events carry: the display name, or the short code when the payload
    /// publishes no display name.
    pub(super) fn source_label(&self) -> &str {
        if self.label.trim().is_empty() {
            self.short.trim()
        } else {
            self.label.trim()
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlatRow {
    /// The result identity (`IDResult`), the same id the athlete-bio payload publishes.
    #[serde(rename = "IDResult")]
    pub result_id: i64,
    #[serde(rename = "AthleteID", default)]
    pub athlete_id: Option<i64>,
    #[serde(rename = "FirstName", default)]
    pub first_name: Option<String>,
    /// Null on relay squad rows, where `FirstName` names the four legs as markup instead.
    #[serde(rename = "LastName", default)]
    pub last_name: Option<String>,
    /// `"11"` for a person; `"-"` on 71 of this capture's 72 relay squad rows.
    #[serde(rename = "Grade", default)]
    pub grade: Option<String>,
    /// The team entry's `IDSchool`.
    #[serde(rename = "TeamID", default)]
    pub team_id: Option<i64>,
    /// The school the row ran for; the team entry keyed by `TeamID` publishes the same name.
    #[serde(rename = "SchoolName", default)]
    pub school_name: String,
    /// `"10.41a"`, `"5-04.25"`, `"DNS"`.
    #[serde(rename = "Result", default)]
    pub result: String,
    /// Published as a string here (empty on a no-mark row).
    #[serde(rename = "Place", default, deserialize_with = "optional_text")]
    pub place: Option<String>,
}

impl FlatRow {
    /// The person the row names, or `None` when it names nobody — a squad row's `FirstName` holds
    /// the legs as markup, not a person.
    pub(super) fn name(&self) -> Option<String> {
        let first = self.first_name.as_deref().unwrap_or_default().trim();
        let last = self.last_name.as_deref().unwrap_or_default().trim();
        let name = format!("{first} {last}");
        let name = name.trim();
        (!name.is_empty()).then(|| name.to_string())
    }
}

/// One relay leg. `ResultID` is the parent relay result (verified on all 288 legs of the capture),
/// so a leg is identified by that result plus its position in the relay — never by `ID`, the leg
/// entry id, which the relay's four legs share.
#[derive(Debug, Clone, Deserialize)]
pub struct PublishedLeg {
    #[serde(rename = "ResultID")]
    pub result_id: i64,
    #[serde(rename = "AthleteID", default)]
    pub athlete_id: Option<i64>,
    #[serde(rename = "Name", default)]
    pub name: String,
    /// The leg's grade (`"11"`) on all 288 legs of the capture.
    #[serde(rename = "ShortDesc", default)]
    pub short_desc: Option<String>,
}

/// One team the payload publishes: its `IDSchool` resolves a row's `TeamID`.
#[derive(Debug, Clone, Deserialize)]
pub struct PublishedTeam {
    #[serde(rename = "IDSchool")]
    pub school_id: i64,
    #[serde(rename = "SchoolName", default)]
    pub school_name: String,
}

/// The metadata document.
#[derive(Debug, Clone, Deserialize)]
pub struct EventDivisions {
    #[serde(rename = "events", default)]
    pub events: Vec<PublishedEvent>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishedEvent {
    /// The event id the results blocks carry as `EventId`.
    #[serde(rename = "ID")]
    pub id: i64,
    /// `"T"` track, `"F"` field.
    #[serde(rename = "Type", default)]
    pub event_type: Option<String>,
    /// `"L"`, `"S"`, `"V"` on this capture's 12 field events; read only by the cross-check.
    #[serde(rename = "FieldMeasureType", default)]
    pub field_measure_type: Option<String>,
    #[serde(rename = "isHurdle", default)]
    pub is_hurdle: bool,
}
