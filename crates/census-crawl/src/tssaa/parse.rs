use census_domain::model::{CoachRole, Gender, Sport};
use census_domain::school_directory::{
    IdentifiedKey, PostalAddress, SchoolDirectoryEntry, SchoolName, SourceLabel, StateRecordId,
};
use census_domain::UsJurisdiction;
use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::directory::{self, compile_pattern, group, line_of, AddressParts, ReadOutcome};
use crate::{CrawlError, CrawlResult};

use super::{fields, postal, staff};

pub struct SchoolRead {
    pub school: ReadOutcome,
    pub coaches: Vec<CoachRow>,
    pub addresses: Vec<(String, PostalAddress)>,
    pub(super) staff_year: Option<super::staff_year::PublishedStaffYear>,
}

pub struct CoachRow {
    pub person: String,
    pub sport: Option<Sport>,
    pub gender: Gender,
    pub role: CoachRole,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub published_role: String,
    pub published_sport: String,
    pub source_staff_id: Option<String>,
}

pub(super) fn source() -> SourceLabel {
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

pub(super) fn artifact(detail: impl Into<String>) -> CrawlError {
    CrawlError::DirectoryArtifact {
        path: PathBuf::from("tssaa artifact"),
        detail: detail.into(),
    }
}

pub fn parse_school_list(text: &str) -> CrawlResult<ReadOutcome> {
    let (start, end) = fields::array_body(text)
        .ok_or_else(|| artifact("the capture embeds no typeahead school array"))?;
    let body = text
        .get(start..end)
        .ok_or_else(|| artifact("invalid school array bounds"))?;
    let pattern = compile_pattern(fields::ARRAY_ENTRY, "tssaa school array entry")?;
    let (mut outcome, _) = pattern.captures_iter(body).fold(
        (ReadOutcome::new(), BTreeSet::new()),
        |(mut outcome, mut seen), captures| {
            let offset = captures
                .get(0)
                .map_or(start, |row| start.saturating_add(row.start()));
            append_index_row(&mut outcome, &mut seen, &captures, line_of(text, offset));
            (outcome, seen)
        },
    );
    if pattern
        .replace_all(body, "")
        .chars()
        .any(|ch| !ch.is_whitespace() && ch != ',')
    {
        outcome.skip(
            line_of(text, start),
            "school array",
            "unparsed indexed school content",
        );
    }
    Ok(outcome)
}

fn append_index_row(
    outcome: &mut ReadOutcome,
    seen: &mut BTreeSet<StateRecordId>,
    captures: &regex::Captures<'_>,
    line: usize,
) {
    let raw_id = group(captures, 1);
    let id = directory::skip_row(outcome, line, "id", numeric_id(raw_id));
    let (name, city) = fields::split_name_and_city(&fields::unescape_js(group(captures, 2)));
    let name = directory::skip_row(outcome, line, "school name", SchoolName::parse(&name));
    let (Some(id), Some(name)) = (id, name) else {
        return;
    };
    if !seen.insert(id.clone()) {
        outcome.skip(
            line,
            "id",
            format!("duplicate array row for {}", id.as_str()),
        );
        return;
    }
    let mut row = SchoolDirectoryEntry::identified(key(&id), source(), Some(name));
    if let Some((city, state)) = city {
        row = row.with_address(directory::skip_absent(
            outcome,
            line,
            directory::postal_address(AddressParts {
                city: &city,
                state: &state,
                ..AddressParts::default()
            }),
        ));
    }
    outcome.push(row);
}

fn numeric_id(raw: &str) -> Result<StateRecordId, census_domain::school_directory::DirectoryError> {
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(
            census_domain::school_directory::DirectoryError::UnsupportedValue {
                field: "TSSAA school id",
                value: raw.to_string(),
            },
        );
    }
    StateRecordId::parse(raw)
}

fn school_name(text: &str) -> CrawlResult<(usize, SchoolName)> {
    let pattern = compile_pattern(fields::H2, "tssaa school heading")?;
    let tag = compile_pattern(fields::TAG, "tssaa tag")?;
    let captures = pattern
        .captures(text)
        .ok_or_else(|| artifact("the page holds no <h2> school name"))?;
    let raw_name = fields::unescape_js(&fields::strip_tags(&tag, group(&captures, 1)));
    let name = SchoolName::parse(&raw_name).map_err(|error| artifact(error.to_string()))?;
    Ok((
        captures.get(0).map_or(1, |row| line_of(text, row.start())),
        name,
    ))
}

pub fn parse_school_page(text: &str, school_id: &StateRecordId) -> CrawlResult<SchoolRead> {
    let (line, name) = school_name(text)?;
    let directory = parse_school_list(text)?;
    let indexed = indexed_owner(&directory, school_id, &name)?;
    let mut school = ReadOutcome::new();
    directory
        .skipped()
        .iter()
        .chain(directory.notes())
        .for_each(|issue| {
            school.note(
                issue.line,
                issue.field,
                format!("detail index: {}", issue.detail),
            );
        });
    let addresses = postal::parse_addresses(text, &mut school)?;
    let staff_year = match super::staff_year::read(text, &name) {
        Ok(claim) => claim,
        Err(error) => {
            school.skip(line, "staff year", error.to_string());
            None
        }
    };
    let address = addresses
        .first()
        .map(|(_, address)| address.clone())
        .or_else(|| indexed.address().cloned());
    let row = SchoolDirectoryEntry::identified(key(school_id), source(), Some(name))
        .with_address(address);
    if row.address().is_none() {
        school.note(
            line,
            "address",
            "no published postal address or indexed city",
        );
    }
    school.push(row);
    let coaches = staff::coach_rows(text, &mut school)?;
    Ok(SchoolRead {
        school,
        coaches,
        addresses,
        staff_year,
    })
}

fn indexed_owner<'a>(
    directory: &'a ReadOutcome,
    school_id: &StateRecordId,
    name: &SchoolName,
) -> CrawlResult<&'a SchoolDirectoryEntry> {
    let indexed = directory.entries().iter().find(|entry| {
        matches!(entry.key(), census_domain::school_directory::DirectoryKey::StateRecord { id, .. }
            if id == school_id)
    }).ok_or_else(|| artifact(format!("school {} is absent from the public index", school_id.as_str())))?;
    if indexed.name() != Some(name) {
        return Err(artifact(format!(
            "school {} detail heading disagrees with its indexed owner",
            school_id.as_str()
        )));
    }
    Ok(indexed)
}
