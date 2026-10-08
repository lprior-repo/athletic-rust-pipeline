use super::{
    census_state, coordinates, first, grade_span, postal_address, read_rows, skip_row,
    AddressParts, FieldFailure,
};
use census_domain::school_directory::{SchoolDirectoryEntry, SchoolName, SourceLabel};
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn checked<T>(result: Result<T, FieldFailure>) -> TestResult<T> {
    result.map_err(|failure| format!("{}: {}", failure.field, failure.error).into())
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
            )?;
            let Some(name) = name else {
                return Ok(());
            };
            let row = SchoolDirectoryEntry::weak(
                name,
                None,
                Some(UsJurisdiction::NewYork),
                SourceLabel::StateEducationAgency {
                    state: UsJurisdiction::NewYork,
                },
            )?;
            outcome.push(row)
        },
    )?;
    let note = outcome.notes().first().ok_or("a note")?;
    check!(eq; note.field, "row");
    check!(eq; outcome.disposition(), crate::CollectionDisposition::Partial);
    let entry = outcome.entries().first().ok_or("retained school")?;
    check!(eq; entry.name(), Some(&SchoolName::parse("A A Kingston Middle School")?));
    Ok(())
}
