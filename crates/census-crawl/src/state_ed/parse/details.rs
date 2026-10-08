use super::{capture_value, fields, matched_line};
use crate::directory::{self, compile_pattern, AddressParts, ReadOutcome};
use crate::CrawlResult;
use census_domain::school_directory::{
    Enrollment, Phone, PostalAddress, SchoolDirectoryEntry, Website,
};

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

pub(super) fn profile_details(
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
