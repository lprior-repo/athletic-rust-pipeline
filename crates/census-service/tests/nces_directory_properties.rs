use census_crawl::directory::{ReadCounts, ReadOutcome};
use census_crawl::nces::{parse_ccd, parse_pss};
use census_crawl::CrawlError;
use census_domain::school_directory::{
    DirectoryKey, SchoolDirectoryEntry, SchoolKind, SourceLabel,
};
use census_domain::UsJurisdiction;

const CCD: &str = include_str!("../../census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv");
const PSS: &str = include_str!("../../census-crawl/tests/fixtures/nces/pss2324_pu_head.csv");

fn entry<'a>(outcome: &'a ReadOutcome, key: &str) -> &'a SchoolDirectoryEntry {
    outcome
        .entries()
        .iter()
        .find(|entry| entry.key().label() == key)
        .unwrap_or_else(|| panic!("no entry with key {key}"))
}

#[test]
fn nces_the_ccd_window_reads_the_same_way_twice() {
    let first = parse_ccd(CCD).expect("the committed capture reads");
    let second = parse_ccd(CCD).expect("the committed capture reads again");
    assert_eq!(first, second);
}

#[test]
fn nces_every_alabama_row_becomes_an_entry_and_alaska_is_a_state_skip() {
    let outcome = parse_ccd(CCD).expect("the committed capture reads");
    assert_eq!(
        outcome.counts(),
        ReadCounts {
            entries: 1557,
            skipped: 42,
            notes: 0
        }
    );
    assert!(outcome
        .skipped()
        .iter()
        .all(|issue| issue.field == "state" && issue.detail.contains("AK")));
    assert_eq!(outcome.skipped().first().expect("a skip").line, 1559);
    assert_eq!(outcome.skipped().last().expect("a skip").line, 1600);
    assert!(outcome
        .entries()
        .iter()
        .all(|entry| entry.sources().contains(&SourceLabel::Ccd)));
}

#[test]
fn nces_the_ccd_reader_maps_identity_address_grades_and_charter_status() {
    let outcome = parse_ccd(CCD).expect("the committed capture reads");

    let middle = entry(&outcome, "nces:010000500870");
    assert_eq!(
        middle.name().map(|name| name.as_str()),
        Some("Albertville Middle School")
    );
    let address = middle.address().expect("address");
    assert_eq!(
        address.line1().map(|line| line.as_str()),
        Some("600 E Alabama Ave")
    );
    assert_eq!(
        address.city().map(|city| city.as_str()),
        Some("Albertville")
    );
    assert_eq!(address.state(), Some(UsJurisdiction::Alabama));
    assert_eq!(
        address.zip().map(|zip| zip.to_string()),
        Some("35950-2336".to_string())
    );
    assert_eq!(
        middle.phone().map(|phone| phone.as_str()),
        Some("(256)878-2341")
    );
    assert_eq!(middle.grades().expect("grades").label(), "7-8");
    assert_eq!(middle.kind(), Some(&SchoolKind::public()));

    let high = entry(&outcome, "nces:010000500871");
    assert_eq!(
        high.name().map(|name| name.as_str()),
        Some("Albertville High School")
    );
    assert_eq!(high.grades().expect("grades").label(), "9-12");
    assert_eq!(
        high.address()
            .and_then(|address| address.zip())
            .map(|zip| zip.to_string()),
        Some("35950-2322".to_string())
    );

    let missing_four = entry(&outcome, "nces:010222001513");
    assert_eq!(
        missing_four
            .address()
            .and_then(|address| address.zip())
            .map(|zip| zip.to_string()),
        Some("35811".to_string())
    );
    assert_eq!(missing_four.grades().expect("grades").label(), "PK-3");

    let ungraded = entry(&outcome, "nces:010000602755");
    assert_eq!(ungraded.grades(), None);
    assert_eq!(
        ungraded
            .address()
            .and_then(|address| address.zip())
            .map(|zip| zip.to_string()),
        Some("35951".to_string())
    );

    let charter = entry(&outcome, "nces:010019702432");
    assert_eq!(charter.kind(), Some(&SchoolKind::charter()));
    assert_eq!(
        charter
            .address()
            .and_then(|address| address.city())
            .map(|city| city.as_str()),
        Some("Mobile")
    );

    let charters = outcome
        .entries()
        .iter()
        .filter(|entry| entry.kind() == Some(&SchoolKind::charter()))
        .count();
    assert_eq!(charters, 28);
    assert!(outcome
        .entries()
        .iter()
        .all(|entry| matches!(entry.key(), DirectoryKey::Nces(_))));
}

#[test]
fn nces_the_ccd_reader_refuses_a_file_without_the_identity_column() {
    let mutated = CCD.replacen("NCESSCH", "SCHOOL_CODE", 1);
    let error = parse_ccd(&mutated).expect_err("the header no longer names the identity column");
    let CrawlError::Invariant { detail } = error else {
        panic!("expected an invariant refusal");
    };
    assert!(detail.contains("NCESSCH"), "{detail}");
}

#[test]
fn nces_the_pss_window_maps_names_addresses_coordinates_and_enrollment() {
    let outcome = parse_pss(PSS).expect("the committed capture reads");
    assert_eq!(
        outcome.counts(),
        ReadCounts {
            entries: 374,
            skipped: 25,
            notes: 0
        }
    );
    assert!(outcome
        .skipped()
        .iter()
        .all(|issue| issue.field == "state" && issue.detail.contains("AK")));
    assert!(outcome
        .entries()
        .iter()
        .all(|entry| entry.sources().contains(&SourceLabel::Pss)));

    let pilgrim = entry(&outcome, "pss:A2380006");
    assert_eq!(
        pilgrim.name().map(|name| name.as_str()),
        Some("MT. PILGRIM CHRISTIAN ACADEMY")
    );
    let address = pilgrim.address().expect("address");
    assert_eq!(
        address.line1().map(|line| line.as_str()),
        Some("6746 Grasselli Rd")
    );
    assert_eq!(address.city().map(|city| city.as_str()), Some("Fairfield"));
    assert_eq!(address.state(), Some(UsJurisdiction::Alabama));
    assert_eq!(
        address.zip().map(|zip| zip.to_string()),
        Some("35064".to_string())
    );
    assert_eq!(
        pilgrim.phone().map(|phone| phone.as_str()),
        Some("2057805096")
    );
    assert_eq!(pilgrim.enrollment().map(|count| count.get()), Some(49));
    assert_eq!(pilgrim.kind(), Some(&SchoolKind::private()));
    let coordinates = pilgrim.coordinates().expect("coordinates");
    assert_eq!(coordinates.latitude().to_string(), "33.46998");
    assert_eq!(coordinates.longitude().to_string(), "-86.924519");
}

#[test]
fn nces_a_quoted_pss_field_keeps_the_comma_it_carries() {
    let outcome = parse_pss(PSS).expect("the committed capture reads");

    let quoted_name = entry(&outcome, "pss:A1970033");
    assert_eq!(
        quoted_name.name().map(|name| name.as_str()),
        Some("COTTAGE HILL CHRISTIAN ACADEMY, LOWER")
    );

    let quoted_street = entry(&outcome, "pss:A0100219");
    assert_eq!(
        quoted_street
            .address()
            .and_then(|address| address.line1())
            .map(|line| line.as_str()),
        Some("130 N St E., Ste C")
    );
}

#[test]
fn nces_the_pss_reader_refuses_a_file_without_the_name_column() {
    let mutated = PSS.replacen("PINST", "SCHOOL_NAME", 1);
    let error = parse_pss(&mutated).expect_err("the header no longer names the school name column");
    let CrawlError::Invariant { detail } = error else {
        panic!("expected an invariant refusal");
    };
    assert!(detail.contains("PINST"), "{detail}");
}

#[test]
fn nces_an_empty_artifact_is_refused_rather_than_read_as_no_rows() {
    let ccd = parse_ccd("").expect_err("an empty artifact is not a ccd file");
    assert!(matches!(ccd, CrawlError::Invariant { .. }));
    let pss = parse_pss("").expect_err("an empty artifact is not a pss file");
    assert!(matches!(pss, CrawlError::Invariant { .. }));
}
