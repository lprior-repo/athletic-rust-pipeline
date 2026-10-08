use census_domain::school_directory::{
    CityName, DirectoryError, Enrollment, Phone, SchoolDirectoryEntry, SchoolName, SourceLabel,
    Website,
};

use crate::directory::{self, AddressParts, CsvRow, Header, ReadOutcome};
use crate::CrawlResult;

pub const TABULAR_REQUIRED: [&str; 3] = ["NAME", "CITY", "STATE"];

fn build_tabular_entry(
    header: &Header,
    record: &CsvRow<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<Option<SchoolDirectoryEntry>, DirectoryError> {
    let name = directory::skip_row(
        outcome,
        line,
        "school name",
        SchoolName::parse(directory::first(
            record,
            header,
            &["NAME", "SCHOOL_NAME", "SCHOOL"],
        )),
    )?;
    let Some(state) = row_state(header, record, line, outcome)? else {
        return Ok(None);
    };
    let Some(name) = name else {
        return Ok(None);
    };
    let city_raw = directory::first(record, header, &["CITY", "TOWN"]);
    let city = directory::skip_failed(
        outcome,
        line,
        directory::field("city", CityName::parse(city_raw)),
    )?;
    match SchoolDirectoryEntry::weak(
        name,
        city,
        Some(state),
        SourceLabel::StateEducationAgency { state },
    ) {
        Ok(entry) => Ok(Some(entry)),
        Err(
            error @ (DirectoryError::Capacity { .. }
            | DirectoryError::Allocation { .. }
            | DirectoryError::Representation { .. }),
        ) => Err(error),
        Err(_) => {
            outcome.skip(line, "school name", "the name leaves no matching form")?;
            Ok(None)
        }
    }
}

fn row_state(
    header: &Header,
    record: &CsvRow<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<Option<census_domain::UsJurisdiction>, DirectoryError> {
    let raw = directory::first(record, header, &["STATE", "STATE_CODE"]);
    if raw.trim().is_empty() {
        outcome.skip(line, "state", "the row states no state")?;
        return Ok(None);
    }
    let state = directory::census_state(raw);
    if state.is_none() {
        outcome.skip(
            line,
            "state",
            directory::issue_detail(format_args!("{} is not a census jurisdiction", raw.trim()))?,
        )?;
    }
    Ok(state)
}

fn enrich_tabular_entry(
    entry: SchoolDirectoryEntry,
    header: &Header,
    record: &CsvRow<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<SchoolDirectoryEntry, DirectoryError> {
    let address = directory::skip_absent(
        outcome,
        line,
        directory::postal_address(AddressParts {
            street: directory::first(record, header, &["STREET", "ADDRESS", "ADDRESS1"]),
            line2: "",
            city: directory::first(record, header, &["CITY", "TOWN"]),
            state: directory::first(record, header, &["STATE", "STATE_CODE"]),
            zip: directory::first(record, header, &["ZIP", "ZIP_CODE", "POSTAL_CODE"]),
            plus4: "",
        }),
    )?;
    enrich_contacts(entry.with_address(address), header, record, line, outcome)
}

fn enrich_contacts(
    entry: SchoolDirectoryEntry,
    header: &Header,
    record: &CsvRow<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<SchoolDirectoryEntry, DirectoryError> {
    let phone = directory::skip_absent(
        outcome,
        line,
        directory::optional(
            "phone",
            Phone::parse(directory::first(record, header, &["PHONE", "PHONE_NUMBER"])),
        ),
    )?;
    let website = directory::skip_absent(
        outcome,
        line,
        directory::optional(
            "website",
            Website::parse(directory::first(record, header, &["WEBSITE", "URL"])),
        ),
    )?;
    let enrollment = directory::skip_absent(
        outcome,
        line,
        directory::optional(
            "enrollment",
            Enrollment::parse(directory::first(
                record,
                header,
                &["ENROLLMENT", "NUMSTUDS"],
            )),
        ),
    )?;
    Ok(entry
        .with_phone(phone)
        .with_website(website)
        .with_enrollment(enrollment))
}

fn tabular_row(
    header: &Header,
    record: &CsvRow<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<(), DirectoryError> {
    let Some(entry) = build_tabular_entry(header, record, line, outcome)? else {
        return Ok(());
    };
    let row = enrich_tabular_entry(entry, header, record, line, outcome)?;
    outcome.push(row)
}

pub fn parse_tabular(text: &str) -> CrawlResult<ReadOutcome> {
    directory::read_rows(
        text,
        "state education agency directory",
        &TABULAR_REQUIRED,
        tabular_row,
    )
}
