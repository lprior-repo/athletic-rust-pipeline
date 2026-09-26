//! The harvest's own vocabulary and its CSV reader: how AthleticLIVE writes a state, a date and a
//! competition level, and the column accessors every row is read through.

use crate::{CrawlError, CrawlResult};
use census_domain::model::CompetitionLevel;
use census_domain::UsJurisdiction;

/// One parsed row of the AthleticLIVE harvest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetRow {
    pub tenant: String,
    pub athleticlive_meet_id: String,
    pub athleticnet_meet_id: Option<String>,
    pub name: String,
    pub city_state: Option<String>,
    pub state_code: UsJurisdiction,
    pub start: String,
    pub end: Option<String>,
    pub has_results: bool,
}

/// True when a date's year is outside a plausible high-school competition window.
///
/// The harvest contains a small tail of corrupt timestamps (for example `2222-08-23`); they are
/// imported as published and counted in the report rather than dropped, so the corruption stays
/// visible downstream.
pub fn implausible_year(date: &str) -> bool {
    match date.get(..4).and_then(|y| y.parse::<u16>().ok()) {
        Some(year) => !(2015..=2030).contains(&year),
        None => true,
    }
}

/// Take the `YYYY-MM-DD` prefix of an ISO timestamp; reject anything else.
fn date_prefix(value: &str) -> Option<String> {
    let head = value.trim().get(..10)?;
    let mut parts = head.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    if !y
        .chars()
        .chain(m.chars())
        .chain(d.chars())
        .all(|c| c.is_ascii_digit())
    {
        return None;
    }
    Some(head.to_string())
}

/// Competition level inferred from meet-name vocabulary.
///
/// Deliberately shallow: only unambiguous markers are honoured, everything else stays `Unknown`
/// rather than guessing. The meet name remains the primary evidence for a later cross-source join.
pub fn infer_level(name: &str) -> CompetitionLevel {
    let lowered = name.to_ascii_lowercase();
    let has = |needle: &str| lowered.contains(needle);
    if has("state") && (has("final") || has("championship") || has("champ") || has("meet")) {
        CompetitionLevel::State
    } else if has("sectional") {
        CompetitionLevel::Sectional
    } else if has("regional") {
        CompetitionLevel::Regional
    } else if has("district") {
        CompetitionLevel::District
    } else if has("conference") || has("conf ") {
        CompetitionLevel::Conference
    } else if has("invitational")
        || has("invite")
        || has("classic")
        || has("relays")
        || has("festival")
        || has("open")
    {
        CompetitionLevel::Invitational
    } else if has("dual") {
        CompetitionLevel::Dual
    } else {
        CompetitionLevel::Unknown
    }
}

/// Identity the harvest's shape errors carry: the CSV is the adapter's source of record and
/// `Options::input` names the operator's copy of it.
const HARVEST_LABEL: &str = "athleticlive harvest CSV";

/// A harvest shape error, tagged with the artifact it was read from.
fn harvest_schema(detail: String) -> CrawlError {
    CrawlError::Schema {
        url: HARVEST_LABEL.to_string(),
        detail,
    }
}

/// One row's column value: the index of `want` resolved against this row's own fields.
fn field<'a>(columns: &[String], fields: &'a [String], want: &str) -> Option<&'a str> {
    columns
        .iter()
        .position(|column| column.eq_ignore_ascii_case(want))
        .and_then(|index| fields.get(index))
        .map(|value| value.trim())
}

/// The columns every harvest row must carry.
fn require_columns(columns: &[String]) -> CrawlResult<()> {
    for required in ["tenant", "athleticlive_meet_id", "name", "state", "start"] {
        let present = columns
            .iter()
            .any(|column| column.eq_ignore_ascii_case(required));
        if !present {
            return Err(harvest_schema(format!(
                "AthleticLIVE CSV is missing required column `{required}`"
            )));
        }
    }
    Ok(())
}

/// A column every row must publish: its trimmed value, or the row's refusal.
fn required_field(
    columns: &[String],
    fields: &[String],
    column: &str,
    line_number: usize,
) -> CrawlResult<String> {
    match field(columns, fields, column) {
        Some(value) if !value.is_empty() => Ok(value.to_string()),
        _ => Err(harvest_schema(format!("row {line_number} has no {column}"))),
    }
}

/// A row's optional text column, blank values dropped.
fn non_empty(columns: &[String], fields: &[String], want: &str) -> Option<String> {
    field(columns, fields, want)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// A row's Athletic.net meet id, when the row publishes one.
fn athleticnet_id(columns: &[String], fields: &[String]) -> Option<String> {
    field(columns, fields, "athleticnet_meet_id")
        .filter(|value| !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()))
        .map(str::to_string)
}

/// Parse the AthleticLIVE harvest CSV. Column order is irrelevant; unknown columns are ignored and
/// the optional columns of the `-seeds` variant are tolerated.
pub fn parse_meets_csv(body: &str) -> CrawlResult<Vec<MeetRow>> {
    let mut lines = body.lines().filter(|line| !line.trim().is_empty());
    let Some(header) = lines.next() else {
        return Err(harvest_schema(
            "AthleticLIVE CSV has no header row".to_string(),
        ));
    };
    let columns = split_csv_record(header);
    require_columns(&columns)?;

    let mut rows = Vec::new();
    for (n, line) in lines.enumerate() {
        let fields = split_csv_record(line);
        if let Some(row) = meet_row(&columns, &fields, n.saturating_add(2))? {
            rows.push(row);
        }
    }
    Ok(rows)
}

/// One harvested row, or `None` when the row publishes no usable state or start date.
fn meet_row(
    columns: &[String],
    fields: &[String],
    line_number: usize,
) -> CrawlResult<Option<MeetRow>> {
    let tenant = required_field(columns, fields, "tenant", line_number)?;
    let meet_id = required_field(columns, fields, "athleticlive_meet_id", line_number)?;
    let name = required_field(columns, fields, "name", line_number)?;
    let Some(state_code) = UsJurisdiction::parse(field(columns, fields, "state").unwrap_or(""))
    else {
        return Ok(None);
    };
    let Some(start) = field(columns, fields, "start").and_then(date_prefix) else {
        return Ok(None);
    };
    let end = field(columns, fields, "end")
        .and_then(date_prefix)
        .filter(|end| end != &start);
    Ok(Some(MeetRow {
        tenant,
        athleticlive_meet_id: meet_id,
        athleticnet_meet_id: athleticnet_id(columns, fields),
        name,
        city_state: non_empty(columns, fields, "city_state"),
        state_code,
        start,
        end,
        has_results: field(columns, fields, "has_results")
            .is_some_and(|value| value.eq_ignore_ascii_case("true")),
    }))
}

/// Minimal RFC 4180 record splitter (the harvest quotes timer-credit HTML, so this is required).
fn split_csv_record(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' => in_quotes = true,
            ',' if !in_quotes => fields.push(std::mem::take(&mut current)),
            other => current.push(other),
        }
    }
    fields.push(current);
    fields
}
