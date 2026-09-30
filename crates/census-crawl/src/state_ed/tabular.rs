use census_domain::school_directory::{
    CityName, Enrollment, Phone, SchoolDirectoryEntry, SchoolName, SourceLabel, Website,
};

use crate::directory::{self, AddressParts, Header, ReadOutcome};
use crate::CrawlResult;

pub const TABULAR_REQUIRED: [&str; 3] = ["NAME", "CITY", "STATE"];

fn build_tabular_entry(
    header: &Header,
    record: &csv::StringRecord,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Option<SchoolDirectoryEntry> {
    let name = directory::skip_row(
        outcome,
        line,
        "school name",
        SchoolName::parse(directory::first(
            record,
            header,
            &["NAME", "SCHOOL_NAME", "SCHOOL"],
        )),
    );
    let state_raw = directory::first(record, header, &["STATE", "STATE_CODE"]);
    if state_raw.trim().is_empty() {
        outcome.skip(line, "state", "the row states no state");
        return None;
    }
    let Some(state) = directory::census_state(state_raw) else {
        outcome.skip(
            line,
            "state",
            format!("{} is not a census jurisdiction", state_raw.trim()),
        );
        return None;
    };
    let name = name?;
    let city_raw = directory::first(record, header, &["CITY", "TOWN"]);
    let city = directory::skip_failed(
        outcome,
        line,
        directory::field("city", CityName::parse(city_raw)),
    );
    let entry = match SchoolDirectoryEntry::weak(
        name,
        city,
        Some(state),
        SourceLabel::StateEducationAgency { state },
    ) {
        Ok(e) => e,
        Err(_) => {
            outcome.skip(line, "school name", "the name leaves no matching form");
            return None;
        }
    };
    Some(entry)
}

fn enrich_tabular_entry(
    entry: SchoolDirectoryEntry,
    header: &Header,
    record: &csv::StringRecord,
    line: usize,
    outcome: &mut ReadOutcome,
) -> SchoolDirectoryEntry {
    let city_raw = directory::first(record, header, &["CITY", "TOWN"]);
    let state_raw = directory::first(record, header, &["STATE", "STATE_CODE"]);
    let mut row = entry;
    row = row.with_address(directory::skip_absent(
        outcome,
        line,
        directory::postal_address(AddressParts {
            street: directory::first(record, header, &["STREET", "ADDRESS", "ADDRESS1"]),
            line2: "",
            city: city_raw,
            state: state_raw,
            zip: directory::first(record, header, &["ZIP", "ZIP_CODE", "POSTAL_CODE"]),
            plus4: "",
        }),
    ));
    row = row.with_phone(directory::skip_absent(
        outcome,
        line,
        directory::optional(
            "phone",
            Phone::parse(directory::first(record, header, &["PHONE", "PHONE_NUMBER"])),
        ),
    ));
    row = row.with_website(directory::skip_absent(
        outcome,
        line,
        directory::optional(
            "website",
            Website::parse(directory::first(record, header, &["WEBSITE", "URL"])),
        ),
    ));
    row = row.with_enrollment(directory::skip_absent(
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
    ));
    row
}

fn tabular_row(
    header: &Header,
    record: &csv::StringRecord,
    line: usize,
    outcome: &mut ReadOutcome,
) {
    let Some(entry) = build_tabular_entry(header, record, line, outcome) else {
        return;
    };
    let row = enrich_tabular_entry(entry, header, record, line, outcome);
    outcome.push(row);
}

pub fn parse_tabular(text: &str) -> CrawlResult<ReadOutcome> {
    directory::read_rows(
        text,
        "state education agency directory",
        &TABULAR_REQUIRED,
        tabular_row,
    )
}
