//! Result-file domain model shared by every vendor format parser.
//!
//! A result file published by a timer (Hy-Tek, RaceDay Scoring, …) is a pile of rows that carry the
//! same facts: a meet, its date, the events inside it, and per-event athlete rows with a grade, a
//! school and a mark. Parsers differ only in how they read the vendor's markup; the adapter that
//! mints canonical entities reads this one shape.

use census_domain::model::{EventKind, Gender, Grade, Mark};

/// One parsed result file.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedMeet {
    /// Meet name from the file header, e.g. `WIAA Track & Field State Championships`.
    pub name: String,
    /// ISO date parsed from the header, e.g. `2025-06-06`.
    pub date: String,
    /// Last day of a multi-day championship, when the header publishes a range.
    pub end_date: Option<String>,
    /// Licensing line naming the timer, e.g. `PrimeTime Timing`.
    pub timer: Option<String>,
    pub events: Vec<ParsedEvent>,
    /// Result rows the parser accepted.
    pub rows_parsed: usize,
    /// Lines that began like a result row but did not satisfy the layout; reported, never guessed.
    pub rows_skipped: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedEvent {
    /// Source label, e.g. `100 Meter Dash`.
    pub label: String,
    pub kind: EventKind,
    pub gender: Gender,
    /// `Division 1`, when the header carries one.
    pub division: Option<String>,
    /// `preliminaries`, `finals`, … — from the section marker that follows the header.
    pub round: Option<String>,
    pub rows: Vec<ParsedRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedRow {
    pub place: Option<u16>,
    /// Empty for relay rows, which name a school rather than an athlete.
    pub name: String,
    pub grade: Option<Grade>,
    pub school: String,
    pub mark: Mark,
    pub wind_mps: Option<f64>,
    /// Heat, flight, or lane number as published.
    pub heat: Option<String>,
    pub points: Option<f64>,
    /// Relay legs, in running order; empty for individual events.
    pub legs: Vec<RelayLeg>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelayLeg {
    pub position: u8,
    pub name: String,
    pub grade: Option<Grade>,
}
