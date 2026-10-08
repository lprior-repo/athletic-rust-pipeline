use super::address::{address_text, split_address, tag_text, AddressText};
use super::{check_size, Patterns};
use crate::directory::{census_state, skip_optional, skip_row, ReadOutcome};
use census_domain::school_directory::{
    CityName, DirectoryError, PostalAddress, SchoolDirectoryEntry, SchoolName, SourceLabel,
    StreetLine, ZipCode,
};

pub(super) fn read_row(
    line: usize,
    chunk: &str,
    patterns: &Patterns,
    label: &SourceLabel,
    outcome: &mut ReadOutcome,
) -> Result<(), DirectoryError> {
    check_size("association row bytes", chunk.len(), 16 * 1024)?;
    let Some(name) = read_name(line, chunk, patterns, outcome)? else {
        return Ok(());
    };
    let address = read_location(line, chunk, patterns, outcome)?;
    let entry = make_entry(name, address, label);
    if let Some(entry) = skip_row(outcome, line, "name", entry)? {
        outcome.push(entry)?;
    }
    Ok(())
}

fn read_name(
    line: usize,
    chunk: &str,
    patterns: &Patterns,
    outcome: &mut ReadOutcome,
) -> Result<Option<SchoolName>, DirectoryError> {
    let raw = patterns
        .h3
        .captures(chunk)
        .and_then(|captures| captures.get(1));
    let raw = raw.map_or("", |inner| inner.as_str());
    let name = tag_text(raw, &patterns.br, &patterns.tag)?;
    check_size("association name bytes", name.len(), 1024)?;
    skip_row(outcome, line, "name", SchoolName::parse(&name))
}

fn read_location(
    line: usize,
    chunk: &str,
    patterns: &Patterns,
    outcome: &mut ReadOutcome,
) -> Result<Option<PostalAddress>, DirectoryError> {
    let raw = address_text(
        chunk,
        &patterns.div,
        &patterns.close_div,
        &patterns.br,
        &patterns.tag,
    )?;
    read_address(line, &split_address(&raw)?, outcome)
}

fn make_entry(
    name: SchoolName,
    address: Option<PostalAddress>,
    label: &SourceLabel,
) -> Result<SchoolDirectoryEntry, DirectoryError> {
    let city = address.as_ref().and_then(|value| value.city()).cloned();
    let state = address.as_ref().and_then(|value| value.state());
    SchoolDirectoryEntry::weak(name, city, state, label.clone())
        .map(|entry| entry.with_address(address))
}

fn read_address(
    line: usize,
    parts: &AddressText<'_>,
    outcome: &mut ReadOutcome,
) -> Result<Option<PostalAddress>, DirectoryError> {
    let line1 = street(parts.street, line, "street", outcome)?;
    let line2 = street(&parts.line2, line, "street line 2", outcome)?;
    let city = city(parts.city, line, outcome)?;
    let zip = zip(parts, line, outcome)?;
    let state = census_state(parts.state);
    if !parts.state.is_empty() && state.is_none() {
        outcome.note(
            line,
            "state",
            "published state is not a census jurisdiction",
        )?;
    }
    Ok(PostalAddress::of(line1, line2, city, state, zip))
}

fn street(
    raw: &str,
    line: usize,
    field: &'static str,
    outcome: &mut ReadOutcome,
) -> Result<Option<StreetLine>, DirectoryError> {
    skip_optional(outcome, line, field, present(raw, StreetLine::parse))
}

fn city(
    raw: &str,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<Option<CityName>, DirectoryError> {
    skip_optional(outcome, line, "city", present(raw, CityName::parse))
}

fn zip(
    parts: &AddressText<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<Option<ZipCode>, DirectoryError> {
    let parsed = present(parts.zip, |code| ZipCode::of(code, Some(parts.plus4)));
    skip_optional(outcome, line, "zip", parsed)
}

fn present<T>(
    value: &str,
    parse: impl FnOnce(&str) -> Result<T, DirectoryError>,
) -> Result<Option<T>, DirectoryError> {
    check_size("association address field bytes", value.len(), 1024)?;
    if value.is_empty() {
        Ok(None)
    } else {
        parse(value).map(Some)
    }
}
