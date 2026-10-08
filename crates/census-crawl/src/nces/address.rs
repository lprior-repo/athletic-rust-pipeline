use crate::directory::{
    cell, issue_detail, skip_optional, AddressParts, CsvRow, Header, ReadOutcome,
};
use census_domain::school_directory::{
    AddressKind, CityName, DirectoryError, PostalAddress, StreetLine, ZipCode,
};
use census_domain::UsJurisdiction;

pub(super) fn ccd(
    header: &Header,
    record: &CsvRow<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<Option<PostalAddress>, DirectoryError> {
    let physical = parts(header, record, AddressKind::Physical);
    let mailing = parts(header, record, AddressKind::Mailing);
    let location_published = published(physical);
    let location = parse(physical, AddressKind::Physical, line, outcome)?;
    let postal = parse(mailing, AddressKind::Mailing, line, outcome)?;
    if location_published {
        if published(mailing) {
            outcome.note(line, "unselected mailing address", issue_detail(format_args!(
                "retained in source capture: street={:?}; line2={:?}; city={:?}; state={:?}; zip={:?}; plus4={:?}",
                mailing.street, mailing.line2, mailing.city, mailing.state, mailing.zip, mailing.plus4,
            ))?)?;
        }
        return Ok(location);
    }
    Ok(postal)
}

fn parts<'a>(header: &Header, record: &'a CsvRow<'_>, kind: AddressKind) -> AddressParts<'a> {
    let names = match kind {
        AddressKind::Physical => ["LSTREET1", "LSTREET2", "LCITY", "LSTATE", "LZIP", "LZIP4"],
        AddressKind::Mailing => ["MSTREET1", "MSTREET2", "MCITY", "MSTATE", "MZIP", "MZIP4"],
        AddressKind::Unknown => [""; 6],
    };
    let [street, line2, city, state, zip, plus4] =
        names.map(|name| cell(record, header.index(name)));
    AddressParts {
        street,
        line2,
        city,
        state,
        zip,
        plus4,
    }
}

fn published(parts: AddressParts<'_>) -> bool {
    [
        parts.street,
        parts.line2,
        parts.city,
        parts.state,
        parts.zip,
        parts.plus4,
    ]
    .into_iter()
    .any(|part| !part.trim().is_empty())
}

pub(super) fn parse(
    parts: AddressParts<'_>,
    kind: AddressKind,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<Option<PostalAddress>, DirectoryError> {
    let line1 = skip_optional(
        outcome,
        line,
        "address street",
        optional(parts.street, StreetLine::parse),
    )?;
    let line2 = skip_optional(
        outcome,
        line,
        "address line2",
        optional(parts.line2, StreetLine::parse),
    )?;
    let city = skip_optional(
        outcome,
        line,
        "address city",
        optional(parts.city, CityName::parse),
    )?;
    let state = parse_state(parts.state, line, outcome)?;
    let zip = parse_zip(parts, line, outcome)?;
    Ok(PostalAddress::of(line1, line2, city, state, zip).map(|address| address.with_kind(kind)))
}

fn optional<T>(
    raw: &str,
    parse: impl FnOnce(&str) -> Result<T, census_domain::school_directory::DirectoryError>,
) -> Result<Option<T>, census_domain::school_directory::DirectoryError> {
    if raw.trim().is_empty() {
        return Ok(None);
    }
    parse(raw).map(Some)
}

fn parse_state(
    raw: &str,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<Option<UsJurisdiction>, DirectoryError> {
    if raw.trim().is_empty() {
        return Ok(None);
    }
    let state = UsJurisdiction::parse(raw);
    if state.is_none() {
        outcome.note(
            line,
            "address state",
            issue_detail(format_args!(
                "unsupported published value {raw:?}; retained in source capture"
            ))?,
        )?;
    }
    Ok(state)
}

fn parse_zip(
    parts: AddressParts<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<Option<ZipCode>, DirectoryError> {
    if parts.zip.trim().is_empty() {
        if !parts.plus4.trim().is_empty() {
            outcome.note(
                line,
                "address zip",
                issue_detail(format_args!(
                    "extension {:?} without base ZIP; retained in source capture",
                    parts.plus4
                ))?,
            )?;
        }
        return Ok(None);
    }
    let parsed = if parts.plus4.trim().is_empty() {
        ZipCode::parse(parts.zip)
    } else {
        ZipCode::of(parts.zip, Some(parts.plus4))
    };
    skip_optional(outcome, line, "address zip", parsed.map(Some))
}
