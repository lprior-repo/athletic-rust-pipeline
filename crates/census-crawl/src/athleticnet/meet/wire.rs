use super::super::parse::optional_text;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MeetData {
    #[serde(rename = "meet")]
    pub meet: PublishedMeet,
    #[serde(rename = "tfDivisions", default)]
    pub divisions: Vec<PublishedDivision>,
    #[serde(rename = "jwtMeet", default)]
    pub token: Option<String>,
    #[serde(rename = "sport2", default)]
    pub sport2: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishedMeet {
    #[serde(rename = "ID")]
    pub id: i64,
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "MeetDate", default)]
    pub date: String,
    #[serde(rename = "EndDate", default)]
    pub end_date: Option<String>,
    #[serde(rename = "SeasonID", default)]
    pub season_id: Option<i16>,
    #[serde(rename = "Location", default)]
    pub location: Option<PublishedLocation>,
    #[serde(rename = "LiveID", default)]
    pub live_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishedLocation {
    #[serde(rename = "Name", default)]
    pub name: String,
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

#[derive(Debug, Clone, Deserialize)]
pub struct AllResults {
    #[serde(rename = "flatEvents", default)]
    pub blocks: Vec<FlatEvent>,
    #[serde(rename = "relayLegs", default)]
    pub legs: Vec<PublishedLeg>,
    #[serde(rename = "teams", default)]
    pub teams: Vec<PublishedTeam>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlatEvent {
    #[serde(rename = "EventId")]
    pub event_id: i64,
    #[serde(rename = "Event", default)]
    pub label: String,
    #[serde(rename = "EventShort", default)]
    pub short: String,
    #[serde(rename = "Gender", default)]
    pub gender: String,
    #[serde(rename = "DivId", default)]
    pub division_id: Option<i64>,
    #[serde(rename = "Division", default)]
    pub division: Option<String>,
    #[serde(rename = "Round", default)]
    pub round: Option<String>,
    #[serde(default)]
    pub results: Vec<FlatRow>,
}

impl FlatEvent {
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
    #[serde(rename = "IDResult")]
    pub result_id: i64,
    #[serde(rename = "AthleteID", default)]
    pub athlete_id: Option<i64>,
    #[serde(rename = "FirstName", default)]
    pub first_name: Option<String>,
    #[serde(rename = "LastName", default)]
    pub last_name: Option<String>,
    #[serde(rename = "Grade", default)]
    pub grade: Option<String>,
    #[serde(rename = "TeamID", default)]
    pub team_id: Option<i64>,
    #[serde(rename = "SchoolName", default)]
    pub school_name: String,
    #[serde(rename = "Result", default)]
    pub result: String,
    #[serde(rename = "Place", default, deserialize_with = "optional_text")]
    pub place: Option<String>,
}

impl FlatRow {
    pub(super) fn name(&self) -> Option<String> {
        let first = self
            .first_name
            .as_deref()
            .map_or(Default::default(), core::convert::identity)
            .trim();
        let last = self
            .last_name
            .as_deref()
            .map_or(Default::default(), core::convert::identity)
            .trim();
        let name = format!("{first} {last}");
        let name = name.trim();
        (!name.is_empty()).then(|| name.to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishedLeg {
    #[serde(rename = "ResultID")]
    pub result_id: i64,
    #[serde(rename = "AthleteID", default)]
    pub athlete_id: Option<i64>,
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "ShortDesc", default)]
    pub short_desc: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishedTeam {
    #[serde(rename = "IDSchool")]
    pub school_id: i64,
    #[serde(rename = "SchoolName", default)]
    pub school_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventDivisions {
    #[serde(rename = "events", default)]
    pub events: Vec<PublishedEvent>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishedEvent {
    #[serde(rename = "ID")]
    pub id: i64,
    #[serde(rename = "Type", default)]
    pub event_type: Option<String>,
    #[serde(rename = "FieldMeasureType", default)]
    pub field_measure_type: Option<String>,
    #[serde(rename = "isHurdle", default)]
    pub is_hurdle: bool,
}
