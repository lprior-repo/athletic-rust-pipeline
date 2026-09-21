//! Published-payload decoding: the wire structs Athletic.net returns, and the token readers
//! that turn a published mark, timing flag, round or grade letter into platform vocabulary.

use crate::sources::hytek::{parse_field_mark, parse_time, NO_MARK};
use census_domain::model::{EventKind, Gender, Mark, Sport, TimingMethod};
use serde::Deserialize;
use std::collections::BTreeMap;

// -------------------------------------------------------------------------------------------------
// Wire format
// -------------------------------------------------------------------------------------------------

/// One athlete bio payload. Fields the adapter does not read are not declared.
#[derive(Debug, Clone, Deserialize)]
pub struct Bio {
    pub athlete: BioAthlete,
    #[serde(default)]
    pub grades: Option<BTreeMap<String, i64>>,
    #[serde(default, rename = "allTeams")]
    pub teams: BTreeMap<String, BioTeam>,
    #[serde(default, rename = "allSeasons")]
    pub seasons: Vec<BioSeason>,
    #[serde(default, rename = "eventsTF")]
    pub events: Option<Vec<BioEvent>>,
    #[serde(default, rename = "resultsTF")]
    pub results_tf: Option<Vec<TfRow>>,
    #[serde(default, rename = "resultsXC")]
    pub results_xc: Option<Vec<XcRow>>,
    #[serde(default)]
    pub meets: BTreeMap<String, BioMeet>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BioAthlete {
    #[serde(rename = "IDAthlete")]
    pub id: u64,
    #[serde(rename = "FirstName", default)]
    pub first_name: String,
    #[serde(rename = "LastName", default)]
    pub last_name: String,
    /// `"M"` or `"F"`.
    #[serde(rename = "Gender", default)]
    pub gender: String,
    #[serde(rename = "SchoolID", default)]
    pub school_id: Option<i64>,
}

impl BioAthlete {
    /// `"First Last"`, trimmed; empty when the payload names nobody.
    pub fn name(&self) -> String {
        format!("{} {}", self.first_name.trim(), self.last_name.trim())
            .trim()
            .to_string()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BioTeam {
    #[serde(rename = "SchoolName", default)]
    pub school_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BioSeason {
    #[serde(rename = "SchoolID", default)]
    pub school_id: i64,
    #[serde(rename = "IDSeason")]
    pub season_id: i16,
    /// `"2026 Indoor"`, `"2026 Outdoor"`, `"2026 Cross Country"`.
    #[serde(rename = "Display", default)]
    pub display: String,
}

impl BioSeason {
    /// The indoor/outdoor split, or `None` when the display does not publish one.
    pub(super) fn sport(&self) -> Option<Sport> {
        let display = self.display.to_ascii_lowercase();
        if display.contains("indoor") {
            Some(Sport::IndoorTrack)
        } else if display.contains("outdoor") {
            Some(Sport::OutdoorTrack)
        } else if display.contains("cross") {
            Some(Sport::CrossCountry)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BioEvent {
    #[serde(rename = "IDEvent")]
    pub id: i64,
    #[serde(rename = "Event", default)]
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BioMeet {
    #[serde(rename = "MeetName", default)]
    pub name: String,
    /// `"2025-05-03T00:00:00"`.
    #[serde(rename = "EndDate", default)]
    pub end_date: String,
}

impl BioMeet {
    pub(super) fn date(&self) -> Option<&str> {
        let date = self.end_date.split('T').next().unwrap_or_default().trim();
        (date.len() == 10 && date.as_bytes().get(4) == Some(&b'-')).then_some(date)
    }
}

/// One track & field result.
#[derive(Debug, Clone, Deserialize)]
pub struct TfRow {
    #[serde(rename = "IDResult")]
    pub id: i64,
    /// `"1:17.80a"` (auto-timed, trailing `a`), `"11.32a"`, `"20.51m"`, `"DNS"`.
    #[serde(rename = "Result", default)]
    pub result: String,
    /// `1` for fully automatic timing.
    #[serde(rename = "FAT", default)]
    pub fat: i64,
    /// Published as a string here and as a number in cross country.
    #[serde(rename = "Place", default, deserialize_with = "optional_text")]
    pub place: Option<String>,
    #[serde(rename = "Round", default)]
    pub round: Option<String>,
    #[serde(rename = "Wind", default)]
    pub wind: Option<f64>,
    #[serde(rename = "Division", default)]
    pub division: Option<String>,
    #[serde(rename = "SchoolID", default)]
    pub school_id: Option<i64>,
    #[serde(rename = "EventID", default)]
    pub event_id: Option<i64>,
    #[serde(rename = "MeetID", default)]
    pub meet_id: Option<i64>,
    #[serde(rename = "SeasonID", default)]
    pub season_id: Option<i16>,
    /// `"2025-05-02T00:00:00"`.
    #[serde(rename = "ResultDate", default)]
    pub result_date: Option<String>,
}

impl TfRow {
    pub(super) fn date(&self) -> Option<String> {
        self.result_date
            .as_deref()
            .and_then(|raw| raw.split('T').next())
            .map(str::trim)
            .filter(|date| date.len() == 10)
            .map(str::to_string)
    }
}

/// One cross-country result. Cross country publishes no event id (the distance replaces it) and no
/// result date (the meet's end date is the published date).
#[derive(Debug, Clone, Deserialize)]
pub struct XcRow {
    #[serde(rename = "IDResult")]
    pub id: i64,
    #[serde(rename = "Result", default)]
    pub result: String,
    #[serde(rename = "Place", default, deserialize_with = "optional_text")]
    pub place: Option<String>,
    #[serde(rename = "Division", default)]
    pub division: Option<String>,
    #[serde(rename = "SchoolID", default)]
    pub school_id: Option<i64>,
    #[serde(rename = "MeetID", default)]
    pub meet_id: Option<i64>,
    #[serde(rename = "SeasonID", default)]
    pub season_id: Option<i16>,
    /// Course distance in metres (`5000`), published beside the cross-country time.
    #[serde(rename = "Distance", default)]
    pub distance: Option<i64>,
}

/// `Place` is a string on the track payload and a number on the cross-country one.
fn optional_text<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(text)) => {
            let text = text.trim().to_string();
            (!text.is_empty()).then_some(text)
        }
        Some(serde_json::Value::Number(number)) => Some(number.to_string()),
        Some(other) => return Err(D::Error::custom(format!("unexpected place value {other}"))),
    })
}

// -------------------------------------------------------------------------------------------------
// Marks
// -------------------------------------------------------------------------------------------------

/// The mark a published result token denotes, plus whether it was fully automatic, or `None` when
/// the token is not a mark at all.
///
/// Athletic.net publishes times and field marks with a trailing `a` when the mark is automatic,
/// qualifier suffixes (`q`/`Q`/`p`/`P`) beside the mark, and no-mark words (`DNS`, `ND`, `FOUL`) in
/// the same column. A no-mark row yields no performance.
pub fn parse_mark(kind: &EventKind, published: &str) -> Option<(Mark, bool)> {
    let trimmed = published.trim();
    if trimmed.is_empty() || NO_MARK.contains(&trimmed.to_ascii_uppercase().as_str()) {
        return None;
    }
    let auto = trimmed.ends_with('a') || trimmed.ends_with('A');
    let token = trimmed
        .trim_end_matches(['a', 'A', 'q', 'Q', 'p', 'P'])
        .trim();
    if !token.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let mark = match kind {
        EventKind::Pentathlon | EventKind::Heptathlon | EventKind::Decathlon => {
            Mark::Points(token.replace(',', "").parse().ok()?)
        }
        kind if kind.is_field() => parse_field_mark(metric_bare(token))?,
        _ => Mark::TimeSeconds(parse_time(token)?),
    };
    Some((mark, auto))
}

/// Drop the `m` Athletic.net prints on metric field marks (`12.34m`), so the shared field parser
/// sees the bare figure it expects. Only a token that is otherwise a plain number is shortened.
fn metric_bare(token: &str) -> &str {
    match token.strip_suffix(['m', 'M']) {
        Some(head) if head.trim().parse::<f64>().is_ok() => head.trim(),
        _ => token,
    }
}

/// Timing method: the published automatic-timing flag and the `a` suffix both mean fully
/// automatic; a bare mark is hand-timed.
pub(super) fn timing_of(fat: i64, auto: bool) -> Option<TimingMethod> {
    if fat == 1 || auto {
        Some(TimingMethod::Fat)
    } else {
        Some(TimingMethod::Hand)
    }
}

pub(super) fn round_of(published: Option<&str>) -> Option<String> {
    let published = published?.trim();
    match published.to_ascii_uppercase().as_str() {
        "" => None,
        "F" => Some("final".to_string()),
        "P" => Some("prelim".to_string()),
        "S" => Some("semifinal".to_string()),
        other => Some(other.to_ascii_lowercase()),
    }
}

pub(super) fn gender_of(published: &str) -> Option<Gender> {
    match published.trim().to_ascii_uppercase().as_str() {
        "F" => Some(Gender::Girls),
        "M" => Some(Gender::Boys),
        _ => None,
    }
}
