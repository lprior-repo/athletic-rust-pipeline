use census_domain::model::{CoachRole, Gender, Sport};
use census_domain::school_directory::{
    IdentifiedKey, SchoolDirectoryEntry, SchoolName, SourceLabel, StateRecordId,
};
use census_domain::UsJurisdiction;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use crate::directory::{self, compile_pattern, group, line_of, AddressParts, ReadOutcome};
use crate::{CrawlError, CrawlResult};

use super::fields;

pub struct SchoolRead {
    pub school: ReadOutcome,
    pub coaches: Vec<CoachRow>,
}

pub struct CoachRow {
    pub person: String,
    pub sport: Option<Sport>,
    pub gender: Gender,
    pub role: CoachRole,
    pub email: Option<String>,
    pub phone: Option<String>,
}

fn source() -> SourceLabel {
    SourceLabel::AthleticAssociation {
        state: UsJurisdiction::Tennessee,
    }
}

fn key(id: &StateRecordId) -> IdentifiedKey {
    IdentifiedKey::StateRecord {
        state: UsJurisdiction::Tennessee,
        id: id.clone(),
    }
}

fn artifact(detail: impl Into<String>) -> CrawlError {
    CrawlError::DirectoryArtifact {
        path: PathBuf::from("tssaa artifact"),
        detail: detail.into(),
    }
}

fn city_of(text: &str, id: &StateRecordId) -> CrawlResult<Option<(String, String)>> {
    let pattern = compile_pattern(fields::ARRAY_ENTRY, "tssaa school array entry")?;
    for captures in pattern.captures_iter(text) {
        if group(&captures, 1) == id.as_str() {
            let (_, city) = fields::split_name_and_city(&fields::unescape_js(group(&captures, 2)));
            return Ok(city);
        }
    }
    Ok(None)
}

pub fn parse_school_list(text: &str) -> CrawlResult<ReadOutcome> {
    let Some((body_start, body_end)) = fields::array_body(text) else {
        return Err(artifact("the capture embeds no typeahead school array"));
    };
    let body = text.get(body_start..body_end).unwrap_or("");
    let pattern = compile_pattern(fields::ARRAY_ENTRY, "tssaa school array entry")?;
    let mut outcome = ReadOutcome::new();
    let mut seen: BTreeSet<StateRecordId> = BTreeSet::new();
    for captures in pattern.captures_iter(body) {
        let offset = captures
            .get(0)
            .map(|matched| body_start.saturating_add(matched.start()))
            .unwrap_or(body_start);
        let line = line_of(text, offset);
        let id = directory::skip_row(
            &mut outcome,
            line,
            "id",
            StateRecordId::parse(group(&captures, 1)),
        );
        let (name_part, city) =
            fields::split_name_and_city(&fields::unescape_js(group(&captures, 2)));
        let name = directory::skip_row(
            &mut outcome,
            line,
            "school name",
            SchoolName::parse(&name_part),
        );
        let (Some(id), Some(name)) = (id, name) else {
            continue;
        };
        if !seen.insert(id.clone()) {
            outcome.skip(
                line,
                "id",
                format!("duplicate array row for {}", id.as_str()),
            );
            continue;
        }
        let mut row = SchoolDirectoryEntry::identified(key(&id), source(), Some(name));
        if let Some((city, state)) = city {
            row = row.with_address(directory::skip_absent(
                &mut outcome,
                line,
                directory::postal_address(AddressParts {
                    street: "",
                    line2: "",
                    city: &city,
                    state: &state,
                    zip: "",
                    plus4: "",
                }),
            ));
        }
        outcome.push(row);
    }
    Ok(outcome)
}

fn school_name(text: &str, tag: &regex::Regex) -> CrawlResult<Option<(usize, String)>> {
    let pattern = compile_pattern(fields::H2, "tssaa school heading")?;
    for captures in pattern.captures_iter(text) {
        let name = fields::strip_tags(tag, group(&captures, 1));
        if name.is_empty() {
            continue;
        }
        let line = captures
            .get(0)
            .map(|matched| line_of(text, matched.start()))
            .unwrap_or(1);
        return Ok(Some((line, name)));
    }
    Ok(None)
}

fn mail_hide_map(text: &str) -> CrawlResult<HashMap<String, String>> {
    let pattern = compile_pattern(fields::MAIL_HIDE, "tssaa mail_hide call")?;
    let mut map: HashMap<String, String> = HashMap::new();
    for captures in pattern.captures_iter(text) {
        if let Some(email) = fields::decode_email(group(&captures, 2), group(&captures, 3)) {
            map.insert(group(&captures, 1).to_string(), email);
        }
    }
    Ok(map)
}

fn card_headers(text: &str) -> CrawlResult<Vec<(usize, String)>> {
    let pattern = compile_pattern(fields::CARD_HEADER, "tssaa card header")?;
    let tag = compile_pattern(fields::TAG, "tssaa tag")?;
    Ok(pattern
        .captures_iter(text)
        .map(|captures| {
            let offset = captures
                .get(0)
                .map(|matched| matched.start())
                .unwrap_or_default();
            (offset, fields::strip_tags(&tag, group(&captures, 1)))
        })
        .collect())
}

fn coach_rows(text: &str) -> CrawlResult<Vec<CoachRow>> {
    let cards = card_headers(text)?;
    let emails = mail_hide_map(text)?;
    let rows = compile_pattern(fields::STAFF_ROW, "tssaa staff row")?;
    let cells = compile_pattern(fields::CELL, "tssaa staff cell")?;
    let staff = compile_pattern(fields::STAFF_ID, "tssaa staff id")?;
    let tag = compile_pattern(fields::TAG, "tssaa tag")?;
    let mut coaches = Vec::new();
    for captures in rows.captures_iter(text) {
        let offset = captures
            .get(0)
            .map(|matched| matched.start())
            .unwrap_or_default();
        let row_html = group(&captures, 1);
        let values: Vec<String> = cells
            .captures_iter(row_html)
            .map(|cell| fields::strip_tags(&tag, group(&cell, 1)))
            .collect();
        let (Some(person), Some(title), Some(phone)) =
            (values.first(), values.get(1), values.get(3))
        else {
            continue;
        };
        let Some(role) = fields::role_of(title) else {
            continue;
        };
        let Some((sport, gender)) = fields::classify(fields::header_for(&cards, offset)) else {
            continue;
        };
        if sport.is_none() && role != CoachRole::AthleticDirector {
            continue;
        }
        if sport.is_some() && role == CoachRole::AthleticDirector {
            continue;
        }
        let email = staff
            .captures(row_html)
            .and_then(|id| emails.get(group(&id, 1)).cloned());
        let phone = phone.trim();
        coaches.push(CoachRow {
            person: person.trim().to_string(),
            sport,
            gender,
            role,
            email,
            phone: if phone.is_empty() {
                None
            } else {
                Some(phone.to_string())
            },
        });
    }
    Ok(coaches)
}

pub fn parse_school_page(text: &str, school_id: &StateRecordId) -> CrawlResult<SchoolRead> {
    let tag = compile_pattern(fields::TAG, "tssaa tag")?;
    let Some((line, raw_name)) = school_name(text, &tag)? else {
        return Err(artifact(format!(
            "the page holds no <h2> school name for id {}",
            school_id.as_str()
        )));
    };
    let mut school = ReadOutcome::new();
    let name = directory::skip_failed(
        &mut school,
        line,
        directory::field(
            "school name",
            SchoolName::parse(&fields::unescape_js(&raw_name)),
        ),
    );
    let mut row = SchoolDirectoryEntry::identified(key(school_id), source(), name);
    match city_of(text, school_id)? {
        Some((city, state)) => {
            row = row.with_address(directory::skip_absent(
                &mut school,
                line,
                directory::postal_address(AddressParts {
                    street: "",
                    line2: "",
                    city: &city,
                    state: &state,
                    zip: "",
                    plus4: "",
                }),
            ));
        }
        None => school.note(
            line,
            "address",
            format!(
                "the embedded array states no city for id {}",
                school_id.as_str()
            ),
        ),
    }
    school.push(row);
    Ok(SchoolRead {
        school,
        coaches: coach_rows(text)?,
    })
}
