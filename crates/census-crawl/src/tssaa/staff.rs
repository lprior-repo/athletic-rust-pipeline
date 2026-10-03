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
    let cards: Vec<_> = headers
        .captures_iter(text)
        .map(|captures| {
            (
                captures.get(0).map_or(0, |row| row.start()),
                fields::strip_tags(&patterns.tag, group(&captures, 1)),
            )
        })
        .collect();
    Ok(patterns
        .rows
        .captures_iter(text)
        .filter_map(|captures| {
            let offset = captures.get(0).map_or(0, |row| row.start());
            let header = fields::header_for(&cards, offset);
            let (sport, gender) = fields::classify(header)?;
            let html = group(&captures, 1);
            let row = parse_row(html, header, sport, gender, &patterns)
                .map_err(|detail| outcome.skip(line_of(text, offset), "appointment", detail))
                .ok()
                .flatten();
            if row.as_ref().is_some_and(|row| row.email.is_none()) && html.contains("mail_hide(") {
                outcome.note(
                    line_of(text, offset),
                    "email",
                    "published mail call is malformed",
                );
            }
            row
        })
        .collect())
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
