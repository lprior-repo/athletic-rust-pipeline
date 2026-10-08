use census_domain::model::CoachRole;

use crate::directory::{compile_pattern, group, line_of, ReadOutcome};
use crate::CrawlResult;

use super::{fields, CoachRow};

struct Patterns {
    rows: regex::Regex,
    cells: regex::Regex,
    staff: regex::Regex,
    tag: regex::Regex,
    mail: regex::Regex,
}

impl Patterns {
    fn new() -> CrawlResult<Self> {
        Ok(Self {
            rows: compile_pattern(fields::STAFF_ROW, "tssaa staff row")?,
            cells: compile_pattern(fields::CELL, "tssaa staff cell")?,
            staff: compile_pattern(fields::STAFF_ID, "tssaa staff id")?,
            tag: compile_pattern(fields::TAG, "tssaa tag")?,
            mail: compile_pattern(fields::MAIL_HIDE, "tssaa public mail call")?,
        })
    }
}

pub(super) fn coach_rows(text: &str, outcome: &mut ReadOutcome) -> CrawlResult<Vec<CoachRow>> {
    let headers = compile_pattern(fields::CARD_HEADER, "tssaa card header")?;
    let patterns = Patterns::new()?;
    let mut cards = Vec::new();
    for captures in headers.captures_iter(text) {
        admit_rows(cards.len())?;
        cards.push((
            captures.get(0).map_or(0, |row| row.start()),
            fields::strip_tags(&patterns.tag, group(&captures, 1)),
        ));
    }
    let mut rows = Vec::new();
    for (count, captures) in patterns.rows.captures_iter(text).enumerate() {
        admit_rows(count)?;
        let offset = captures.get(0).map_or(0, |row| row.start());
        let header = fields::header_for(&cards, offset);
        if let Some((sport, gender)) = fields::classify(header) {
            append_row(
                &mut rows,
                outcome,
                group(&captures, 1),
                (header, sport, gender, line_of(text, offset)),
                &patterns,
            )?;
        }
    }
    Ok(rows)
}

fn admit_rows(count: usize) -> CrawlResult<()> {
    if count >= 20_000 {
        return Err(crate::CrawlError::Resource {
            resource: "TSSAA staff rows",
            requested: count.saturating_add(1),
            limit: 20_000,
        });
    }
    Ok(())
}

fn append_row(
    rows: &mut Vec<CoachRow>,
    outcome: &mut ReadOutcome,
    html: &str,
    context: (
        &str,
        Option<census_domain::model::Sport>,
        census_domain::model::Gender,
        usize,
    ),
    patterns: &Patterns,
) -> CrawlResult<()> {
    let (header, sport, gender, line) = context;
    let row = match parse_row(html, header, sport, gender, patterns) {
        Ok(row) => row,
        Err(error) => {
            outcome.skip(line, "appointment", error)?;
            None
        }
    };
    if row.as_ref().is_some_and(|row| row.email.is_none()) && html.contains("mail_hide(") {
        outcome.note(line, "email", "published mail call is malformed")?;
    }
    if let Some(row) = row {
        rows.push(row);
    }
    Ok(())
}

fn parse_row(
    html: &str,
    header: &str,
    sport: Option<census_domain::model::Sport>,
    gender: census_domain::model::Gender,
    patterns: &Patterns,
) -> Result<Option<CoachRow>, String> {
    if html.contains("<th") {
        return Ok(None);
    }
    let values: Vec<_> = patterns
        .cells
        .captures_iter(html)
        .map(|cell| fields::strip_tags(&patterns.tag, group(&cell, 1)))
        .collect();
    let (Some(person), Some(title)) = (values.first(), values.get(1)) else {
        return Err("appointment table row has no person/title cells".to_string());
    };
    let role = fields::role_of(title).map_or(CoachRole::Unknown, |role| role);
    if sport.is_none() && role != CoachRole::AthleticDirector {
        return Ok(None);
    }
    if role == CoachRole::AthleticDirector && sport.is_some() {
        return Err("athletic director appears inside a sport appointment table".to_string());
    }
    if person.is_empty() {
        return Err("published appointment has no person".to_string());
    }
    let email = patterns
        .mail
        .captures(html)
        .and_then(|mail| fields::decode_email(group(&mail, 2), group(&mail, 3)));
    let phone = values.get(3).filter(|value| !value.is_empty()).cloned();
    let source_staff_id = patterns
        .staff
        .captures(html)
        .map(|id| group(&id, 1).to_string());
    Ok(Some(CoachRow {
        person: person.clone(),
        sport,
        gender,
        role,
        email,
        phone,
        published_role: title.clone(),
        published_sport: header.to_string(),
        source_staff_id,
    }))
}
