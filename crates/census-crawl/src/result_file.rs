
use census_domain::model::{EventKind, Gender, Grade, Mark};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedMeet {
    pub name: String,
    pub date: String,
    pub end_date: Option<String>,
    pub timer: Option<String>,
    pub events: Vec<ParsedEvent>,
    pub rows_parsed: usize,
    pub rows_skipped: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedEvent {
    pub label: String,
    pub kind: EventKind,
    pub gender: Gender,
    pub division: Option<String>,
    pub round: Option<String>,
    pub rows: Vec<ParsedRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedRow {
    pub place: Option<u16>,
    pub name: String,
    pub grade: Option<Grade>,
    pub school: String,
    pub mark: Mark,
    pub wind_mps: Option<f64>,
    pub heat: Option<String>,
    pub points: Option<f64>,
    pub legs: Vec<RelayLeg>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelayLeg {
    pub position: u8,
    pub name: String,
    pub grade: Option<Grade>,
}
