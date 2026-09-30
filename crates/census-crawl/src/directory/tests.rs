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

fn sample_entry(name: &str) -> SchoolDirectoryEntry {
    SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010001000001").expect("nces id")),
        SourceLabel::Ccd,
        Some(SchoolName::parse(name).expect("school name")),
    )
}

#[test]
fn outcomes_count_entries_skips_and_notes() {
    let mut outcome = ReadOutcome::new();
    outcome.push(sample_entry("Albertville High School"));
    outcome.skip(7, "nces id", "12 digits expected");
    outcome.note(9, "grades", "09..13 has no rankable grade");

    assert_eq!(
        outcome.counts(),
        ReadCounts {
            entries: 1,
            skipped: 1,
            notes: 1
        }
    );
    assert_eq!(outcome.entries().len(), 1);
    assert_eq!(
        outcome.skipped().first().map(RowIssue::render),
        Some("line 7: nces id 12 digits expected".to_string())
    );
    assert_eq!(outcome.notes().first().expect("note").line, 9);
}

#[test]
fn outcomes_absorb_each_other_without_losing_an_issue() {
    let mut first = ReadOutcome::new();
    first.push(sample_entry("Albertville High School"));
    first.skip(2, "row", "empty name");
    let mut second = ReadOutcome::new();
    second.note(2, "phone", "12345 is not a phone");
    second.push(sample_entry("Boaz High School"));

    first.absorb(second);
    assert_eq!(first.counts().entries, 2);
    assert_eq!(first.counts().skipped, 1);
    assert_eq!(first.counts().notes, 1);
    assert_eq!(first.into_entries().len(), 2);
}

#[test]
fn field_failures_carry_the_field_and_the_domain_detail() {
    let failure = field("zip", ZipCode::parse("3595")).expect_err("five digits are required");
    assert_eq!(failure.field, "zip");
    assert!(failure.detail.contains("3595"));

    let present = field("zip", ZipCode::parse("35950")).expect("five digits parse");
    assert_eq!(present.code(), "35950");

    let missing = optional("phone", Phone::parse("   ")).expect("blank phones are absent");
    assert_eq!(missing, None);

    let failure =
        optional("enrollment", Enrollment::parse("1,200")).expect_err("a comma is not a digit");
    assert_eq!(
        failure,
        FieldFailure {
            field: "enrollment",
            detail: "enrollment is not an integer: \"1,200\"".to_string()
        }
    );
}

#[test]
fn postal_addresses_carry_the_parts_the_row_has() {
    let full = postal_address(AddressParts {
        street: "600 E Alabama Ave",
        line2: "Suite 4",
        city: "Albertville",
        state: "AL",
        zip: "35950",
        plus4: "2336",
    })
    .expect("address parses")
    .expect("address is present");
    assert_eq!(
        full.line1().map(|line| line.as_str()),
        Some("600 E Alabama Ave")
    );
    assert_eq!(full.line2().map(|line| line.as_str()), Some("Suite 4"));
    assert_eq!(full.city().map(|city| city.as_str()), Some("Albertville"));
    assert_eq!(full.state(), Some(UsJurisdiction::Alabama));
    assert_eq!(
        full.zip().map(|zip| zip.to_string()),
        Some("35950-2336".to_string())
    );

    let locality = postal_address(AddressParts {
        street: "",
        line2: "",
        city: "Albertville",
        state: "AL",
        zip: "35950",
        plus4: "",
    })
    .expect("locality parses")
    .expect("locality is present");
    assert_eq!(locality.line1(), None);
    assert_eq!(locality.state(), Some(UsJurisdiction::Alabama));

    let absent = postal_address(AddressParts {
        street: " ",
        ..AddressParts::default()
    })
    .expect("blank street is absent");
    assert_eq!(absent, None);

    let nine = postal_address(AddressParts {
        street: "335 Homer Nance Road",
        city: "Huntsville",
        state: "AL",
        zip: "358112336",
        ..AddressParts::default()
    })
    .expect("a nine-digit zip parses")
    .expect("address");
    assert_eq!(
        nine.zip().map(|zip| zip.to_string()),
        Some("35811-2336".to_string())
    );
    assert_eq!(nine.line2(), None);

    let plus4_only = postal_address(AddressParts {
        street: "534 Industrial Rd",
        city: "Alabaster",
        state: "AL",
        plus4: "9106",
        ..AddressParts::default()
    })
    .expect("a plus4 without a five-digit zip is absent")
    .expect("address");
    assert_eq!(plus4_only.zip(), None);
}

#[test]
fn row_mapping_helpers_split_skips_from_notes() {
    let mut outcome = ReadOutcome::new();
    let kept = skip_row(
        &mut outcome,
        3,
        "ncessch",
        NcesSchoolId::parse("010001000001"),
    );
    assert!(kept.is_some());
    let dropped = skip_row(
        &mut outcome,
        4,
        "ncessch",
        NcesSchoolId::parse("0100010000"),
    );
    assert_eq!(dropped, None);
    let absent = skip_optional(&mut outcome, 5, "phone", Phone::parse("  "));
    assert_eq!(absent, None);
    let failed = skip_failed(
        &mut outcome,
        6,
        postal_address(AddressParts {
            street: "600 E Alabama Ave",
            zip: "3595",
            ..AddressParts::default()
        }),
    );
    assert_eq!(failed, None);
    assert_eq!(outcome.counts().skipped, 1);
    assert_eq!(outcome.counts().notes, 1);
    assert_eq!(outcome.skipped().first().expect("skip").field, "ncessch");
    assert_eq!(outcome.notes().first().expect("note").field, "zip");
}

#[test]
fn postal_addresses_refuse_malformed_parts_and_out_of_scope_states() {
    let failure = postal_address(AddressParts {
        street: "600 E Alabama Ave",
        state: "AL",
        zip: "35950-2336-1",
        ..AddressParts::default()
    })
    .expect_err("three zip segments are malformed");
    assert_eq!(failure.field, "zip");

    let foreign = postal_address(AddressParts {
        street: "1 Main St",
        state: "HI",
        zip: "96813",
        ..AddressParts::default()
    })
    .expect("Hawaii parses as a state")
    .expect("an address is present");
    assert_eq!(foreign.state(), None);
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
fn grade_spans_map_the_published_codes() {
    let span = grade_span("09", "12").expect("ranks parse").expect("span");
    assert_eq!(span.label(), "9-12");
    let primary = grade_span("PK", "05").expect("ranks parse").expect("span");
    assert_eq!(primary.label(), "PK-5");
    let kindergarten = grade_span("KG", "KG").expect("ranks parse").expect("span");
    assert_eq!(kindergarten.label(), "KG-KG");

    assert_eq!(grade_span("M", "M").expect("missing is absent"), None);
    assert_eq!(grade_span("N", "").expect("not applicable is absent"), None);

    let failure = grade_span("09", "13").expect_err("grade 13 has no rank");
    assert_eq!(failure.field, "grades");
    assert!(grade_span("UG", "UG").is_err());
    assert!(grade_span("12", "09").is_err());
}

#[test]
fn coordinates_parse_only_when_both_axes_arrive() {
    let present = coordinates("33.46998", "-86.924519")
        .expect("both axes parse")
        .expect("coordinates");
    assert_eq!(present.latitude().to_string(), "33.46998");
    assert_eq!(coordinates("", "").expect("both axes blank"), None);
    assert!(coordinates("33.46998", "").is_err());
    assert!(coordinates("91", "-86.9").is_err());
}

#[test]
fn a_row_shorter_than_the_header_is_counted_as_a_note() {
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
            );
            let Some(name) = name else {
                return;
            };
            let row = SchoolDirectoryEntry::weak(
                name,
                None,
                Some(UsJurisdiction::NewYork),
                SourceLabel::StateEducationAgency {
                    state: UsJurisdiction::NewYork,
                },
            )
            .expect("a weak key");
            outcome.push(row);
        },
    )
    .expect("the documented header reads");
    assert_eq!(outcome.entries().len(), 1);
    assert_eq!(outcome.counts().notes, 1);
    let note = outcome.notes().first().expect("a note");
    assert_eq!(note.field, "row");
    assert!(
        note.detail.contains("3 of 4 cells"),
        "the note names the truncation: {}",
        note.detail
    );
}
