use crate::directory::{
    cell, census_state, coordinates, first, grade_span, read_rows, skip_absent, skip_optional,
    skip_row, AddressParts, CsvRow, Header, ReadOutcome,
};
use crate::CrawlResult;
use census_domain::school_directory::{
    AddressKind, DirectoryError, Enrollment, IdentifiedKey, NcesSchoolId, Phone, PssId,
    SchoolDirectoryEntry, SchoolKind, SchoolName, SourceLabel, Website,
};

#[path = "address.rs"]
mod address;

struct Row<'a> {
    header: &'a Header,
    record: &'a CsvRow<'a>,
    line: usize,
    state: &'a str,
}

pub const CCD_REQUIRED: [&str; 11] = [
    "NCESSCH",
    "SCH_NAME",
    "MSTREET1",
    "MCITY",
    "MSTATE",
    "MZIP",
    "MZIP4",
    "PHONE",
    "GSLO",
    "GSHI",
    "CHARTER_TEXT",
];

pub const PSS_REQUIRED: [&str; 9] = [
    "PPIN", "PINST", "PADDRS", "PCITY", "PSTABB", "PZIP", "PZIP4", "PPHONE", "NUMSTUDS",
];

fn process_ccd_row(
    header: &Header,
    record: &CsvRow<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<(), DirectoryError> {
    let Some((id, name, state)) = ccd_identity(header, record, line, outcome)? else {
        return Ok(());
    };
    let entry = ccd_entry(
        Row {
            header,
            record,
            line,
            state,
        },
        id,
        name,
        outcome,
    )?;
    outcome.push(entry)
}

fn ccd_identity<'a>(
    header: &Header,
    record: &'a CsvRow<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<Option<(NcesSchoolId, SchoolName, &'a str)>, DirectoryError> {
    let Some(id) = skip_row(
        outcome,
        line,
        "ncessch",
        NcesSchoolId::parse(cell(record, header.index("NCESSCH"))),
    )?
    else {
        return Ok(None);
    };
    let Some(name) = skip_row(
        outcome,
        line,
        "school name",
        SchoolName::parse(cell(record, header.index("SCH_NAME"))),
    )?
    else {
        return Ok(None);
    };
    let state = first(record, header, &["ST", "LSTATE", "MSTATE"]);
    if !state_admitted(outcome, line, state)? {
        return Ok(None);
    }
    Ok(Some((id, name, state)))
}

fn ccd_entry(
    row: Row<'_>,
    id: NcesSchoolId,
    name: SchoolName,
    outcome: &mut ReadOutcome,
) -> Result<SchoolDirectoryEntry, DirectoryError> {
    let address = address::ccd(row.header, row.record, row.line, outcome)?;
    let kind = if cell(row.record, row.header.index("CHARTER_TEXT"))
        .trim()
        .eq_ignore_ascii_case("yes")
    {
        SchoolKind::charter()
    } else {
        SchoolKind::public()
    };
    let grades = skip_absent(
        outcome,
        row.line,
        grade_span(
            cell(row.record, row.header.index("GSLO")),
            cell(row.record, row.header.index("GSHI")),
        ),
    )?;
    let entry =
        SchoolDirectoryEntry::identified(IdentifiedKey::Nces(id), SourceLabel::Ccd, Some(name))
            .with_jurisdiction(census_state(row.state))
            .with_address(address)
            .with_kind(Some(kind))
            .with_grades(grades);
    ccd_contacts(entry, row, outcome)
}

fn ccd_contacts(
    entry: SchoolDirectoryEntry,
    row: Row<'_>,
    outcome: &mut ReadOutcome,
) -> Result<SchoolDirectoryEntry, DirectoryError> {
    let phone = skip_optional(
        outcome,
        row.line,
        "phone",
        Phone::parse(cell(row.record, row.header.index("PHONE"))),
    )?;
    let website = skip_optional(
        outcome,
        row.line,
        "website",
        Website::parse(first(row.record, row.header, &["WEBSITE"])),
    )?;
    let enrollment = skip_optional(
        outcome,
        row.line,
        "enrollment",
        Enrollment::parse(cell(row.record, row.header.index("ENROLLMENT"))),
    )?;
    Ok(entry
        .with_phone(phone)
        .with_website(website)
        .with_enrollment(enrollment))
}

pub fn parse_ccd(text: &str) -> CrawlResult<ReadOutcome> {
    read_rows(text, "ccd", &CCD_REQUIRED, process_ccd_row)
}

fn process_pss_row(
    header: &Header,
    record: &CsvRow<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<(), DirectoryError> {
    let Some((id, name, state)) = pss_identity(header, record, line, outcome)? else {
        return Ok(());
    };
    let entry = pss_entry(
        Row {
            header,
            record,
            line,
            state,
        },
        id,
        name,
        outcome,
    )?;
    outcome.push(entry)
}

fn pss_identity<'a>(
    header: &Header,
    record: &'a CsvRow<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<Option<(PssId, SchoolName, &'a str)>, DirectoryError> {
    let Some(id) = skip_row(
        outcome,
        line,
        "ppin",
        PssId::parse(cell(record, header.index("PPIN"))),
    )?
    else {
        return Ok(None);
    };
    let Some(name) = skip_row(
        outcome,
        line,
        "school name",
        SchoolName::parse(cell(record, header.index("PINST"))),
    )?
    else {
        return Ok(None);
    };
    let state = cell(record, header.index("PSTABB"));
    if !state_admitted(outcome, line, state)? {
        return Ok(None);
    }
    Ok(Some((id, name, state)))
}

fn pss_entry(
    row: Row<'_>,
    id: PssId,
    name: SchoolName,
    outcome: &mut ReadOutcome,
) -> Result<SchoolDirectoryEntry, DirectoryError> {
    let address = address::parse(
        AddressParts {
            street: cell(row.record, row.header.index("PADDRS")),
            city: cell(row.record, row.header.index("PCITY")),
            state: row.state,
            zip: cell(row.record, row.header.index("PZIP")),
            plus4: cell(row.record, row.header.index("PZIP4")),
            ..AddressParts::default()
        },
        AddressKind::Mailing,
        row.line,
        outcome,
    )?;
    let entry =
        SchoolDirectoryEntry::identified(IdentifiedKey::Pss(id), SourceLabel::Pss, Some(name))
            .with_jurisdiction(census_state(row.state))
            .with_address(address)
            .with_kind(Some(SchoolKind::private()));
    pss_contacts(entry, row, outcome)
}

fn pss_contacts(
    entry: SchoolDirectoryEntry,
    row: Row<'_>,
    outcome: &mut ReadOutcome,
) -> Result<SchoolDirectoryEntry, DirectoryError> {
    let phone = skip_optional(
        outcome,
        row.line,
        "phone",
        Phone::parse(cell(row.record, row.header.index("PPHONE"))),
    )?;
    let enrollment = skip_optional(
        outcome,
        row.line,
        "enrollment",
        Enrollment::parse(cell(row.record, row.header.index("NUMSTUDS"))),
    )?;
    let coordinates = skip_absent(
        outcome,
        row.line,
        coordinates(
            cell(row.record, row.header.index("LATITUDE24")),
            cell(row.record, row.header.index("LONGITUDE24")),
        ),
    )?;
    Ok(entry
        .with_phone(phone)
        .with_enrollment(enrollment)
        .with_coordinates(coordinates))
}

fn state_admitted(
    outcome: &mut ReadOutcome,
    line: usize,
    state: &str,
) -> Result<bool, DirectoryError> {
    if census_state(state).is_some() {
        return Ok(true);
    }
    outcome.skip(
        line,
        "state",
        crate::directory::issue_detail(format_args!("{state} is not a census jurisdiction"))?,
    )?;
    Ok(false)
}

pub fn parse_pss(text: &str) -> CrawlResult<ReadOutcome> {
    read_rows(text, "pss", &PSS_REQUIRED, process_pss_row)
}
