use super::{
    census_state, coordinates, field, first, grade_span, optional, postal_address, read_rows,
    skip_failed, skip_optional, skip_row, AddressParts, FieldFailure, ReadCounts, ReadOutcome,
    RowIssue,
};
use census_domain::school_directory::{
    Enrollment, IdentifiedKey, NcesSchoolId, Phone, SchoolDirectoryEntry, SchoolName, SourceLabel,
    ZipCode,
};
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn checked<T>(result: Result<T, FieldFailure>) -> TestResult<T> {
    result.map_err(|failure| format!("{}: {}", failure.field, failure.detail).into())
}

fn sample_entry(name: &str) -> TestResult<SchoolDirectoryEntry> {
    Ok(SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010001000001")?),
        SourceLabel::Ccd,
        Some(SchoolName::parse(name)?),
    ))
}

#[test]
fn outcomes_count_entries_skips_and_notes() -> TestResult {
    let mut outcome = ReadOutcome::new();
    outcome.push(sample_entry("Albertville High School")?);
    outcome.skip(7, "nces id", "12 digits expected");
    outcome.note(9, "grades", "09..13 has no rankable grade");

    check!(eq;
        outcome.counts(),
        ReadCounts {
            entries: 1,
            skipped: 1,
            notes: 1
        }
    );
    check!(eq; outcome.entries().len(), 1);
    check!(eq;
        outcome.skipped().first().map(RowIssue::render),
        Some("line 7: nces id 12 digits expected".to_string())
    );
    check!(eq; outcome.notes().first().ok_or("note")?.line, 9);
    Ok(())
}

#[test]
fn outcomes_absorb_each_other_without_losing_an_issue() -> TestResult {
    let mut first = ReadOutcome::new();
    first.push(sample_entry("Albertville High School")?);
    first.skip(2, "row", "empty name");
    let mut second = ReadOutcome::new();
    second.note(2, "phone", "12345 is not a phone");
    second.push(sample_entry("Boaz High School")?);

    first.absorb(second);
    check!(eq; first.counts().entries, 2);
    check!(eq; first.counts().skipped, 1);
    check!(eq; first.counts().notes, 1);
    check!(eq; first.into_entries().len(), 2);
    Ok(())
}

#[test]
fn field_failures_carry_the_field_and_the_domain_detail() -> TestResult {
    let failure = match field("zip", ZipCode::parse("3595")) {
        Err(failure) => failure,
        Ok(_) => return Err("five digits are required".into()),
    };
    check!(eq; failure.field, "zip");
    check!(failure.detail.contains("3595"));

    let present = checked(field("zip", ZipCode::parse("35950")))?;
    check!(eq; present.code(), "35950");

    let missing = checked(optional("phone", Phone::parse("   ")))?;
    check!(eq; missing, None);

    let failure = match optional("enrollment", Enrollment::parse("1,200")) {
        Err(failure) => failure,
        Ok(_) => return Err("a comma is not a digit".into()),
    };
    check!(eq;
        failure,
        FieldFailure {
            field: "enrollment",
            detail: "enrollment is not an integer: \"1,200\"".to_string()
        }
    );
    Ok(())
}

#[test]
fn postal_addresses_carry_the_parts_the_row_has() -> TestResult {
    let full = checked(postal_address(AddressParts {
        street: "600 E Alabama Ave",
        line2: "Suite 4",
        city: "Albertville",
        state: "AL",
        zip: "35950",
        plus4: "2336",
    }))?
    .ok_or("address is present")?;
    check!(eq;
        full.line1().map(|line| line.as_str()),
        Some("600 E Alabama Ave")
    );
    check!(eq; full.line2().map(|line| line.as_str()), Some("Suite 4"));
    check!(eq; full.city().map(|city| city.as_str()), Some("Albertville"));
    check!(eq; full.state(), Some(UsJurisdiction::Alabama));
    check!(eq;
        full.zip().map(|zip| zip.to_string()),
        Some("35950-2336".to_string())
    );

    let locality = checked(postal_address(AddressParts {
        street: "",
        line2: "",
        city: "Albertville",
        state: "AL",
        zip: "35950",
        plus4: "",
    }))?
    .ok_or("locality is present")?;
    check!(eq; locality.line1(), None);
    check!(eq; locality.state(), Some(UsJurisdiction::Alabama));

    let absent = checked(postal_address(AddressParts {
        street: " ",
        ..AddressParts::default()
    }))?;
    check!(eq; absent, None);

    let nine = checked(postal_address(AddressParts {
        street: "335 Homer Nance Road",
        city: "Huntsville",
        state: "AL",
        zip: "358112336",
        ..AddressParts::default()
    }))?
    .ok_or("address")?;
    check!(eq;
        nine.zip().map(|zip| zip.to_string()),
        Some("35811-2336".to_string())
    );
    check!(eq; nine.line2(), None);

    let plus4_only = checked(postal_address(AddressParts {
        street: "534 Industrial Rd",
        city: "Alabaster",
        state: "AL",
        plus4: "9106",
        ..AddressParts::default()
    }))?
    .ok_or("address")?;
    check!(eq; plus4_only.zip(), None);
    Ok(())
}

#[test]
fn row_mapping_helpers_split_skips_from_notes() -> TestResult {
    let mut outcome = ReadOutcome::new();
    let kept = skip_row(
        &mut outcome,
        3,
        "ncessch",
        NcesSchoolId::parse("010001000001"),
    );
    check!(kept.is_some());
    let dropped = skip_row(
        &mut outcome,
        4,
        "ncessch",
        NcesSchoolId::parse("0100010000"),
    );
    check!(eq; dropped, None);
    let absent = skip_optional(&mut outcome, 5, "phone", Phone::parse("  "));
    check!(eq; absent, None);
    let failed = skip_failed(
        &mut outcome,
        6,
        postal_address(AddressParts {
            street: "600 E Alabama Ave",
            zip: "3595",
            ..AddressParts::default()
        }),
    );
    check!(eq; failed, None);
    check!(eq; outcome.counts().skipped, 1);
    check!(eq; outcome.counts().notes, 1);
    check!(eq; outcome.skipped().first().ok_or("skip")?.field, "ncessch");
    check!(eq; outcome.notes().first().ok_or("note")?.field, "zip");
    Ok(())
}

#[test]
fn postal_addresses_refuse_malformed_parts_and_out_of_scope_states() -> TestResult {
    let failure = match postal_address(AddressParts {
        street: "600 E Alabama Ave",
        state: "AL",
        zip: "35950-2336-1",
        ..AddressParts::default()
    }) {
        Err(failure) => failure,
        Ok(_) => return Err("three zip segments are malformed".into()),
    };
    check!(eq; failure.field, "zip");

    let foreign = checked(postal_address(AddressParts {
        street: "1 Main St",
        state: "HI",
        zip: "96813",
        ..AddressParts::default()
    }))?
    .ok_or("an address is present")?;
    check!(eq; foreign.state(), None);
    Ok(())
}

#[test]
fn census_states_are_the_forty_nine_planned_jurisdictions() {
    assert_eq!(census_state("AL"), Some(UsJurisdiction::Alabama));
    assert_eq!(census_state(" ny "), Some(UsJurisdiction::NewYork));
    assert_eq!(census_state("Tennessee"), Some(UsJurisdiction::Tennessee));
    assert_eq!(census_state("HI"), None);
    assert_eq!(census_state("AK"), None);
    assert_eq!(census_state("PR"), None);
    assert_eq!(census_state(""), None);
}

#[test]
fn grade_spans_map_the_published_codes() -> TestResult {
    let span = checked(grade_span("09", "12"))?.ok_or("span")?;
    check!(eq; span.label(), "9-12");
    let primary = checked(grade_span("PK", "05"))?.ok_or("span")?;
    check!(eq; primary.label(), "PK-5");
    let kindergarten = checked(grade_span("KG", "KG"))?.ok_or("span")?;
    check!(eq; kindergarten.label(), "KG-KG");

    check!(eq; checked(grade_span("M", "M"))?, None);
    check!(eq; checked(grade_span("N", ""))?, None);

    let failure = match grade_span("09", "13") {
        Err(failure) => failure,
        Ok(_) => return Err("grade 13 has no rank".into()),
    };
    check!(eq; failure.field, "grades");
    check!(grade_span("UG", "UG").is_err());
    check!(grade_span("12", "09").is_err());
    Ok(())
}

#[test]
fn coordinates_parse_only_when_both_axes_arrive() -> TestResult {
    let present = checked(coordinates("33.46998", "-86.924519"))?.ok_or("coordinates")?;
    check!(eq; present.latitude().to_string(), "33.46998");
    check!(eq; checked(coordinates("", ""))?, None);
    check!(coordinates("33.46998", "").is_err());
    check!(coordinates("91", "-86.9").is_err());
    Ok(())
}

#[test]
fn a_row_shorter_than_the_header_is_counted_as_a_note() -> TestResult {
    let text = "NAME,CITY,STATE,PHONE\nA A Kingston Middle School,Potsdam,NY\n";
    let mut mapping_error = None;
    let outcome = read_rows(
        text,
        "test directory",
        &["NAME", "CITY", "STATE"],
        |header, record, line, outcome| {
            let name = skip_row(
                outcome,
                line,
                "school name",
                SchoolName::parse(first(record, header, &["NAME"])),
            );
            let Some(name) = name else {
                return;
            };
            let row = match SchoolDirectoryEntry::weak(
                name,
                None,
                Some(UsJurisdiction::NewYork),
                SourceLabel::StateEducationAgency {
                    state: UsJurisdiction::NewYork,
                },
            ) {
                Ok(row) => row,
                Err(error) => {
                    mapping_error = Some(error);
                    return;
                }
            };
            outcome.push(row);
        },
    )?;
    if let Some(error) = mapping_error {
        return Err(error.into());
    }
    check!(eq; outcome.entries().len(), 1);
    check!(eq; outcome.counts().notes, 1);
    let note = outcome.notes().first().ok_or("a note")?;
    check!(eq; note.field, "row");
    check!(
        note.detail.contains("3 of 4 cells"),
        "the note names the truncation: {}",
        note.detail
    );
    Ok(())
}
