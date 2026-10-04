use crate::directory::{
    cell, census_state, coordinates, first, grade_span, postal_address, read_rows, skip_absent,
    skip_optional, skip_row, AddressParts, Header, ReadOutcome,
};
use crate::CrawlResult;
use census_domain::school_directory::{
    Enrollment, IdentifiedKey, NcesSchoolId, Phone, PssId, SchoolDirectoryEntry, SchoolKind,
    SchoolName, SourceLabel, Website,
};

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
    record: &csv::StringRecord,
    line: usize,
    outcome: &mut ReadOutcome,
) {
    let Some((id, name, state)) = ccd_identity(header, record, line, outcome) else {
        return;
    };
    let entry = ccd_entry(header, record, line, state, id, name, outcome);
    outcome.push(entry);
}

fn ccd_identity<'a>(
    header: &Header,
    record: &'a csv::StringRecord,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Option<(NcesSchoolId, SchoolName, &'a str)> {
    let id = skip_row(
        outcome,
        line,
        "ncessch",
        NcesSchoolId::parse(cell(record, header.index("NCESSCH"))),
    )?;
    let name = skip_row(
        outcome,
        line,
        "school name",
        SchoolName::parse(cell(record, header.index("SCH_NAME"))),
    )?;
    let state = first(record, header, &["LSTATE", "MSTATE", "ST"]);
    if !state_admitted(outcome, line, state) {
        return None;
    }
    Some((id, name, state))
}

fn ccd_entry(
    header: &Header,
    record: &csv::StringRecord,
    line: usize,
    state: &str,
    id: NcesSchoolId,
    name: SchoolName,
    outcome: &mut ReadOutcome,
) -> SchoolDirectoryEntry {
    let address = skip_absent(
        outcome,
        line,
        postal_address(AddressParts {
            street: first(record, header, &["LSTREET1", "MSTREET1"]),
            line2: first(record, header, &["LSTREET2", "MSTREET2"]),
            city: first(record, header, &["LCITY", "MCITY"]),
            state,
            zip: first(record, header, &["LZIP", "MZIP"]),
            plus4: first(record, header, &["LZIP4", "MZIP4"]),
        }),
    );
    let phone = skip_optional(
        outcome,
        line,
        "phone",
        Phone::parse(cell(record, header.index("PHONE"))),
    );
    let website = skip_optional(
        outcome,
        line,
        "website",
        Website::parse(first(record, header, &["WEBSITE"])),
    );
    let enrollment = skip_optional(
        outcome,
        line,
        "enrollment",
        Enrollment::parse(cell(record, header.index("ENROLLMENT"))),
    );
    let grades = skip_absent(
        outcome,
        line,
        grade_span(
            cell(record, header.index("GSLO")),
            cell(record, header.index("GSHI")),
        ),
    );
    let charter = cell(record, header.index("CHARTER_TEXT"))
        .trim()
        .eq_ignore_ascii_case("yes");
    let kind = if charter {
        SchoolKind::charter()
    } else {
        SchoolKind::public()
    };
    SchoolDirectoryEntry::identified(IdentifiedKey::Nces(id), SourceLabel::Ccd, Some(name))
        .with_address(address)
        .with_kind(Some(kind))
        .with_grades(grades)
        .with_enrollment(enrollment)
        .with_phone(phone)
        .with_website(website)
}

pub fn parse_ccd(text: &str) -> CrawlResult<ReadOutcome> {
    read_rows(text, "ccd", &CCD_REQUIRED, process_ccd_row)
}

fn process_pss_row(
    header: &Header,
    record: &csv::StringRecord,
    line: usize,
    outcome: &mut ReadOutcome,
) {
    let Some((id, name, state)) = pss_identity(header, record, line, outcome) else {
        return;
    };
    let entry = pss_entry(header, record, line, state, id, name, outcome);
    outcome.push(entry);
}

fn pss_identity<'a>(
    header: &Header,
    record: &'a csv::StringRecord,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Option<(PssId, SchoolName, &'a str)> {
    let id = skip_row(
        outcome,
        line,
        "ppin",
        PssId::parse(cell(record, header.index("PPIN"))),
    )?;
    let name = skip_row(
        outcome,
        line,
        "school name",
        SchoolName::parse(cell(record, header.index("PINST"))),
    )?;
    let state = cell(record, header.index("PSTABB"));
    if !state_admitted(outcome, line, state) {
        return None;
    }
    Some((id, name, state))
}

fn pss_entry(
    header: &Header,
    record: &csv::StringRecord,
    line: usize,
    state: &str,
    id: PssId,
    name: SchoolName,
    outcome: &mut ReadOutcome,
) -> SchoolDirectoryEntry {
    let address = skip_absent(
        outcome,
        line,
        postal_address(AddressParts {
            street: cell(record, header.index("PADDRS")),
            city: cell(record, header.index("PCITY")),
            state,
            zip: cell(record, header.index("PZIP")),
            plus4: cell(record, header.index("PZIP4")),
            ..AddressParts::default()
        }),
    );
    let phone = skip_optional(
        outcome,
        line,
        "phone",
        Phone::parse(cell(record, header.index("PPHONE"))),
    );
    let enrollment = skip_optional(
        outcome,
        line,
        "enrollment",
        Enrollment::parse(cell(record, header.index("NUMSTUDS"))),
    );
    let coordinates = skip_absent(
        outcome,
        line,
        coordinates(
            cell(record, header.index("LATITUDE24")),
            cell(record, header.index("LONGITUDE24")),
        ),
    );
    SchoolDirectoryEntry::identified(IdentifiedKey::Pss(id), SourceLabel::Pss, Some(name))
        .with_address(address)
        .with_kind(Some(SchoolKind::private()))
        .with_enrollment(enrollment)
        .with_phone(phone)
        .with_coordinates(coordinates)
}

fn state_admitted(outcome: &mut ReadOutcome, line: usize, state: &str) -> bool {
    if census_state(state).is_some() {
        return true;
    }
    outcome.skip(
        line,
        "state",
        format!("{state} is not a census jurisdiction"),
    );
    false
}

pub fn parse_pss(text: &str) -> CrawlResult<ReadOutcome> {
    read_rows(text, "pss", &PSS_REQUIRED, process_pss_row)
}
