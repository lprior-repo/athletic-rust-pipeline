use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MeetsEnvelope {
    pub count: u32,
    pub data: Vec<MeetRow>,
}

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
    #[serde(rename = "LastRefreshedAt")]
    pub last_refreshed_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventsEnvelope {
    #[serde(rename = "meetId")]
    pub meet_id: u64,
    pub count: u32,
    pub data: Vec<EventRow>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventRow {
    #[serde(rename = "eventId")]
    pub event_id: String,
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub gender: String,
    #[serde(rename = "classDivision")]
    pub class_division: String,
    #[serde(rename = "eventName")]
    pub event_name: String,
    pub round: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "hasResults")]
    pub has_results: bool,
    #[serde(rename = "scheduledDate")]
    pub scheduled_date: Option<String>,
    #[serde(rename = "metadataFetchedAt")]
    pub metadata_fetched_at: Option<String>,
}

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
    #[serde(default)]
    pub finishers: Vec<FinisherRow>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FinisherRow {
    pub place: Option<u16>,
    pub mark: Option<String>,
    #[serde(rename = "athleteName")]
    pub athlete_name: Option<String>,
    pub year: Option<String>,
    #[serde(rename = "teamName")]
    pub team_name: Option<String>,
    #[serde(rename = "ihsaSchoolId")]
    pub ihsa_school_id: Option<String>,
    pub athlete: Option<AthleteRef>,
    pub team: Option<TeamRef>,
    #[serde(default)]
    pub members: Vec<RelayMember>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AthleteRef {
    #[serde(rename = "athleticNetId")]
    pub athletic_net_id: Option<u64>,
    #[serde(rename = "athleticLiveId")]
    pub athletic_live_id: Option<u64>,
    pub name: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
    pub year: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TeamRef {
    #[serde(rename = "athleticNetId")]
    pub athletic_net_id: Option<u64>,
    #[serde(rename = "athleticLiveId")]
    pub athletic_live_id: Option<u64>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelayMember {
    pub order: Option<u32>,
    pub athlete: Option<AthleteRef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QualifiersEnvelope {
    #[serde(rename = "tournamentId")]
    pub tournament_id: String,
    #[serde(rename = "boxAssignments", default)]
    pub box_assignments: Vec<BoxRow>,
    #[serde(rename = "teamQualifiers", default)]
    pub team_qualifiers: Vec<QualifierTeam>,
    #[serde(rename = "individualQualifiers", default)]
    pub individual_qualifiers: Vec<QualifierTeam>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BoxRow {
    #[serde(rename = "Box")]
    pub box_id: Option<String>,
    #[serde(rename = "Type")]
    pub entry_type: Option<String>,
    #[serde(rename = "SchoolName")]
    pub school_name: Option<String>,
    #[serde(rename = "FirstName")]
    pub first_name: Option<String>,
    #[serde(rename = "LastName")]
    pub last_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QualifierTeam {
    #[serde(rename = "schoolName")]
    pub school_name: Option<String>,
    #[serde(rename = "ihsaSchoolId")]
    pub ihsa_school_id: Option<String>,
    #[serde(rename = "teamPlace")]
    pub team_place: Option<u32>,
    #[serde(default)]
    pub athletes: Vec<QualifierAthlete>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QualifierAthlete {
    pub number: Option<u32>,
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
    #[serde(rename = "yearInSchool")]
    pub year_in_school: Option<String>,
    #[serde(rename = "box")]
    pub r#box: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TermsEnvelope {
    #[serde(rename = "currentTerm")]
    pub current_term: String,
    #[serde(default)]
    pub terms: Vec<TermRow>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TermRow {
    pub term: String,
    #[serde(default)]
    pub routes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorEnvelope {
    pub error: String,
}
