use census_domain::school_directory::{
    DirectoryError, Enrollment, IdentifiedKey, Phone, PostalAddress, SchoolDirectoryEntry,
    SchoolName, SourceLabel, StateRecordId, Website,
};
use census_domain::UsJurisdiction;
use regex::Regex;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::directory::{self, compile_pattern, group, line_of, AddressParts, ReadOutcome};
use crate::{CrawlError, CrawlResult};

use super::fields;

const NYSED: UsJurisdiction = UsJurisdiction::NewYork;

fn source() -> SourceLabel {
    SourceLabel::StateEducationAgency { state: NYSED }
}

fn artifact(detail: impl Into<String>) -> CrawlError {
    CrawlError::DirectoryArtifact {
        path: PathBuf::from("state_ed artifact"),
        detail: detail.into(),
    }
}

fn matched_line(pattern: &Regex, text: &str) -> usize {
    pattern
        .captures(text)
        .and_then(|captures| captures.get(0))
        .map(|matched| line_of(text, matched.start()))
        .map_or(1, |value| value)
}

fn capture_value<'a>(pattern: &Regex, text: &'a str) -> Result<&'a str, DirectoryError> {
    let raw = pattern
        .captures(text)
        .map(|captures| group(&captures, 1))
        .map_or("", |value| value);
    bounded_field(raw)
}

fn bounded_field(raw: &str) -> Result<&str, DirectoryError> {
    if raw.len() > 4096 {
        return Err(DirectoryError::Capacity {
            resource: "NYSED field bytes",
            requested: raw.len(),
            limit: 4096,
        });
    }
    Ok(raw)
}

pub fn parse_index(text: &str) -> CrawlResult<ReadOutcome> {
    let mut outcome = ReadOutcome::new();
    if let Err(error) = body_limit(text) {
        outcome.stop(1, error);
        return Ok(outcome);
    }
    let pattern = compile_pattern(fields::INDEX_ROW, "NYSED index row")?;
    match walk_index(&pattern, text, &mut outcome) {
        Ok(()) => outcome.finish(),
        Err((line, error)) => outcome.stop(line, error),
    }
    Ok(outcome)
}

fn walk_index(
    pattern: &Regex,
    text: &str,
    outcome: &mut ReadOutcome,
) -> Result<(), (usize, DirectoryError)> {
    let mut seen = HashSet::new();
    let mut rows = pattern.captures_iter(text).peekable();
    if rows.peek().is_none() {
        return Err((
            1,
            DirectoryError::Representation {
                detail: directory::issue_detail(format_args!(
                    "NYSED index has no published school rows"
                ))
                .map_err(|error| (1, error))?,
            },
        ));
    }
    rows.take(20_001)
        .enumerate()
        .try_for_each(|(index, captures)| {
            let line = captures
                .get(0)
                .map(|matched| line_of(text, matched.start()))
                .map_or(1, |value| value);
            if index >= 20_000 {
                return Err((
                    line,
                    DirectoryError::Capacity {
                        resource: "NYSED index rows",
                        requested: 20_001,
                        limit: 20_000,
                    },
                ));
            }
            index_row(&captures, line, &mut seen, outcome).map_err(|error| (line, error))
        })
}

fn index_row<'a>(
    captures: &regex::Captures<'a>,
    line: usize,
    seen: &mut HashSet<&'a str>,
    outcome: &mut ReadOutcome,
) -> Result<(), DirectoryError> {
    let raw = bounded_field(group(captures, 1))?;
    let id = directory::skip_row(outcome, line, "institution id", StateRecordId::parse(raw))?;
    let name = directory::skip_row(
        outcome,
        line,
        "school name",
        SchoolName::parse(bounded_field(group(captures, 2))?),
    )?;
    let (Some(id), Some(name)) = (id, name) else {
        return Ok(());
    };
    if seen.contains(raw.trim()) {
        return outcome.skip(
            line,
            "institution id",
            directory::issue_detail(format_args!("duplicate row for {}", id.as_str()))?,
        );
    }
    seen.try_reserve(1)
        .map_err(|_| DirectoryError::Allocation {
            resource: "NYSED index IDs",
        })?;
    seen.insert(raw.trim());
    let key = IdentifiedKey::StateRecord { state: NYSED, id };
    outcome.push(SchoolDirectoryEntry::identified(key, source(), Some(name)))
}

fn phone(text: &str, outcome: &mut ReadOutcome) -> CrawlResult<Option<Phone>> {
    let pattern = compile_pattern(fields::PHONE, "NYSED phone row")?;
    let line = matched_line(&pattern, text);
    let raw = capture_value(&pattern, text)?;
    Ok(directory::skip_absent(
        outcome,
        line,
        directory::optional("phone", Phone::parse(raw)),
    )?)
}

fn website(text: &str, outcome: &mut ReadOutcome) -> CrawlResult<Option<Website>> {
    let pattern = compile_pattern(fields::WEBSITE, "NYSED website row")?;
    let line = matched_line(&pattern, text);
    let raw = capture_value(&pattern, text)?;
    Ok(directory::skip_absent(
        outcome,
        line,
        directory::optional("website", Website::parse(raw)),
    )?)
}

fn enrollment(text: &str, outcome: &mut ReadOutcome) -> CrawlResult<Option<Enrollment>> {
    let pattern = compile_pattern(fields::TOTAL_STUDENTS, "NYSED enrollment row")?;
    let line = matched_line(&pattern, text);
    let raw = capture_value(&pattern, text)?;
    if raw.is_empty() {
        outcome.note(
            line,
            "enrollment",
            "the page publishes no total_students figure",
        )?;
        return Ok(None);
    }
    Ok(directory::skip_absent(
        outcome,
        line,
        directory::optional("enrollment", Enrollment::parse(raw)),
    )?)
}

fn address(text: &str, outcome: &mut ReadOutcome) -> CrawlResult<Option<PostalAddress>> {
    let pattern = compile_pattern(fields::MAPS_QUERY, "NYSED map query")?;
    let line = matched_line(&pattern, text);
    let raw = capture_value(&pattern, text)?;
    if raw.is_empty() {
        outcome.note(
            line,
            "address",
            "the page embeds no map query, so it carries no address",
        )?;
        return Ok(None);
    }
    let decoded = fields::percent_decode(raw)?;
    map_address(&decoded, line, outcome)
}

fn map_address(
    decoded: &str,
    line: usize,
    outcome: &mut ReadOutcome,
) -> CrawlResult<Option<PostalAddress>> {
    let mut parts = decoded.split(',').map(str::trim);
    let (Some(street), Some(city), Some(state), Some(zip)) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        outcome.note(
            line,
            "address",
            directory::issue_detail(format_args!(
                "the map query {decoded} does not hold street, city, state and zip"
            ))?,
        )?;
        return Ok(None);
    };
    if parts.next().is_some() {
        outcome.note(
            line,
            "address",
            directory::issue_detail(format_args!(
                "the map query {decoded} holds more parts than street, city, state and zip"
            ))?,
        )?;
        return Ok(None);
    }
    mapped_postal([street, city, state, zip], line, outcome)
}

fn mapped_postal(
    parts: [&str; 4],
    line: usize,
    outcome: &mut ReadOutcome,
) -> CrawlResult<Option<PostalAddress>> {
    let [street, city, state, zip] = parts;
    Ok(directory::skip_absent(
        outcome,
        line,
        directory::postal_address(AddressParts {
            street,
            line2: "",
            city,
            state,
            zip,
            plus4: "",
        }),
    )?)
}

pub fn parse_profile(text: &str) -> CrawlResult<ReadOutcome> {
    let mut outcome = ReadOutcome::new();
    if let Err(error) = body_limit(text) {
        outcome.stop(1, error);
        return Ok(outcome);
    }
    match read_profile(text, &mut outcome) {
        Ok(()) => outcome.finish(),
        Err(CrawlError::Directory(error)) => outcome.stop(1, error),
        Err(error) => return Err(error),
    }
    Ok(outcome)
}

fn read_profile(text: &str, outcome: &mut ReadOutcome) -> CrawlResult<()> {
    let Some(row) = profile_identity(text, outcome)? else {
        return Ok(());
    };
    let row = profile_details(row, text, outcome)?;
    outcome.push(row)?;
    Ok(())
}

fn body_limit(text: &str) -> Result<(), DirectoryError> {
    const LIMIT: usize = 8 * 1024 * 1024;
    if text.len() > LIMIT {
        return Err(DirectoryError::Capacity {
            resource: "NYSED body bytes",
            requested: text.len(),
            limit: LIMIT,
        });
    }
    Ok(())
}

fn profile_identity(
    text: &str,
    outcome: &mut ReadOutcome,
) -> CrawlResult<Option<SchoolDirectoryEntry>> {
    let title = compile_pattern(fields::PROFILE_TITLE, "NYSED profile title")?;
    let name_raw = required_capture(
        &title,
        text,
        "the page states no NYSED school title, so it is not a school profile",
    )?;
    let id_pattern = compile_pattern(fields::INSTITUTION_ID, "NYSED institution id")?;
    let id_raw = required_capture(
        &id_pattern,
        text,
        "the page states no institution id, so it is not a school profile",
    )?;
    let line = matched_line(&id_pattern, text);
    let Some(id) = directory::skip_row(
        outcome,
        line,
        "institution id",
        StateRecordId::parse(id_raw),
    )?
    else {
        return Ok(None);
    };
    let name = directory::skip_failed(
        outcome,
        line,
        directory::field("school name", SchoolName::parse(name_raw)),
    )?;
    let key = IdentifiedKey::StateRecord { state: NYSED, id };
    Ok(Some(SchoolDirectoryEntry::identified(key, source(), name)))
}

fn required_capture<'a>(pattern: &Regex, text: &'a str, missing: &str) -> CrawlResult<&'a str> {
    let raw = capture_value(pattern, text)?;
    if raw.is_empty() {
        return Err(artifact(missing));
    }
    Ok(raw)
}

fn profile_details(
    mut row: SchoolDirectoryEntry,
    text: &str,
    outcome: &mut ReadOutcome,
) -> CrawlResult<SchoolDirectoryEntry> {
    row = row.with_phone(phone(text, outcome)?);
    row = row.with_website(website(text, outcome)?);
    row = row.with_address(address(text, outcome)?);
    row = row.with_enrollment(enrollment(text, outcome)?);
    Ok(row)
}
