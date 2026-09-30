use census_domain::school_directory::{
    Enrollment, IdentifiedKey, Phone, PostalAddress, SchoolDirectoryEntry, SchoolName, SourceLabel,
    StateRecordId, Website,
};
use census_domain::UsJurisdiction;
use regex::Regex;
use std::collections::BTreeSet;
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
        .unwrap_or(1)
}

fn capture_value(pattern: &Regex, text: &str) -> String {
    pattern
        .captures(text)
        .map(|captures| group(&captures, 1))
        .unwrap_or("")
        .to_string()
}

pub fn parse_index(text: &str) -> CrawlResult<ReadOutcome> {
    let pattern = compile_pattern(fields::INDEX_ROW, "NYSED index row")?;
    let mut outcome = ReadOutcome::new();
    let mut seen: BTreeSet<StateRecordId> = BTreeSet::new();
    for captures in pattern.captures_iter(text) {
        let line = captures
            .get(0)
            .map(|matched| line_of(text, matched.start()))
            .unwrap_or(1);
        let id = directory::skip_row(
            &mut outcome,
            line,
            "institution id",
            StateRecordId::parse(group(&captures, 1)),
        );
        let name = directory::skip_row(
            &mut outcome,
            line,
            "school name",
            SchoolName::parse(group(&captures, 2)),
        );
        let (Some(id), Some(name)) = (id, name) else {
            continue;
        };
        if !seen.insert(id.clone()) {
            outcome.skip(
                line,
                "institution id",
                format!("duplicate row for {}", id.as_str()),
            );
            continue;
        }
        let key = IdentifiedKey::StateRecord { state: NYSED, id };
        outcome.push(SchoolDirectoryEntry::identified(key, source(), Some(name)));
    }
    Ok(outcome)
}

fn phone(text: &str, outcome: &mut ReadOutcome) -> CrawlResult<Option<Phone>> {
    let pattern = compile_pattern(fields::PHONE, "NYSED phone row")?;
    let line = matched_line(&pattern, text);
    let raw = capture_value(&pattern, text);
    Ok(directory::skip_absent(
        outcome,
        line,
        directory::optional("phone", Phone::parse(&raw)),
    ))
}

fn website(text: &str, outcome: &mut ReadOutcome) -> CrawlResult<Option<Website>> {
    let pattern = compile_pattern(fields::WEBSITE, "NYSED website row")?;
    let line = matched_line(&pattern, text);
    let raw = capture_value(&pattern, text);
    Ok(directory::skip_absent(
        outcome,
        line,
        directory::optional("website", Website::parse(&raw)),
    ))
}

fn enrollment(text: &str, outcome: &mut ReadOutcome) -> CrawlResult<Option<Enrollment>> {
    let pattern = compile_pattern(fields::TOTAL_STUDENTS, "NYSED enrollment row")?;
    let line = matched_line(&pattern, text);
    let raw = capture_value(&pattern, text);
    if raw.is_empty() {
        outcome.note(
            line,
            "enrollment",
            "the page publishes no total_students figure",
        );
        return Ok(None);
    }
    Ok(directory::skip_absent(
        outcome,
        line,
        directory::optional("enrollment", Enrollment::parse(&raw)),
    ))
}

fn address(text: &str, outcome: &mut ReadOutcome) -> CrawlResult<Option<PostalAddress>> {
    let pattern = compile_pattern(fields::MAPS_QUERY, "NYSED map query")?;
    let line = matched_line(&pattern, text);
    let raw = capture_value(&pattern, text);
    if raw.is_empty() {
        outcome.note(
            line,
            "address",
            "the page embeds no map query, so it carries no address",
        );
        return Ok(None);
    }
    let decoded = fields::percent_decode(&raw);
    let mut parts = decoded.split(',').map(str::trim);
    let (Some(street), Some(city), Some(state), Some(zip)) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        outcome.note(
            line,
            "address",
            format!("the map query {decoded} does not hold street, city, state and zip"),
        );
        return Ok(None);
    };
    if parts.next().is_some() {
        outcome.note(
            line,
            "address",
            format!("the map query {decoded} holds more parts than street, city, state and zip"),
        );
        return Ok(None);
    }
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
    ))
}

pub fn parse_profile(text: &str) -> CrawlResult<ReadOutcome> {
    let title = compile_pattern(fields::PROFILE_TITLE, "NYSED profile title")?;
    let name_raw = capture_value(&title, text);
    if name_raw.is_empty() {
        return Err(artifact(
            "the page states no NYSED school title, so it is not a school profile",
        ));
    }
    let id_pattern = compile_pattern(fields::INSTITUTION_ID, "NYSED institution id")?;
    let id_raw = capture_value(&id_pattern, text);
    if id_raw.is_empty() {
        return Err(artifact(
            "the page states no institution id, so it is not a school profile",
        ));
    }
    let line = matched_line(&id_pattern, text);
    let mut outcome = ReadOutcome::new();
    let Some(id) = directory::skip_row(
        &mut outcome,
        line,
        "institution id",
        StateRecordId::parse(&id_raw),
    ) else {
        return Ok(outcome);
    };
    let name = directory::skip_failed(
        &mut outcome,
        line,
        directory::field("school name", SchoolName::parse(&name_raw)),
    );
    let key = IdentifiedKey::StateRecord { state: NYSED, id };
    let mut row = SchoolDirectoryEntry::identified(key, source(), name);
    row = row.with_phone(phone(text, &mut outcome)?);
    row = row.with_website(website(text, &mut outcome)?);
    row = row.with_address(address(text, &mut outcome)?);
    row = row.with_enrollment(enrollment(text, &mut outcome)?);
    outcome.push(row);
    Ok(outcome)
}
