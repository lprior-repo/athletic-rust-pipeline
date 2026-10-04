#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_crawl::directory::{ReadCounts, ReadOutcome};
use census_crawl::nces::{parse_ccd, parse_pss};
use census_crawl::CrawlError;
use census_domain::school_directory::{
    DirectoryKey, SchoolDirectoryEntry, SchoolKind, SourceLabel,
};
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CCD: &str = include_str!("../../census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv");
const PSS: &str = include_str!("../../census-crawl/tests/fixtures/nces/pss2324_pu_head.csv");

fn entry<'a>(outcome: &'a ReadOutcome, key: &str) -> TestResult<&'a SchoolDirectoryEntry> {
    outcome
        .entries()
        .iter()
        .find(|entry| entry.key().label() == key)
        .ok_or_else(|| format!("no entry with key {key}").into())
}

#[test]
fn nces_the_ccd_window_reads_the_same_way_twice() -> TestResult {
    let first = parse_ccd(CCD)?;
    let second = parse_ccd(CCD)?;
    check!(eq; first, second);
    Ok(())
}

#[test]
fn nces_every_alabama_row_becomes_an_entry_and_alaska_is_a_state_skip() -> TestResult {
    let outcome = parse_ccd(CCD)?;
    check!(eq;
        outcome.counts(),
        ReadCounts {
            entries: 1557,
            skipped: 42,
            notes: 15
        }
    );
    check!(outcome
        .notes()
        .iter()
        .all(|issue| issue.field == "website"));
    check!(outcome
        .skipped()
        .iter()
        .all(|issue| issue.field == "state" && issue.detail.contains("AK")));
    check!(eq; outcome.skipped().first().ok_or("missing first skip")?.line, 1559);
    check!(eq; outcome.skipped().last().ok_or("missing last skip")?.line, 1600);
    check!(outcome
        .entries()
        .iter()
        .all(|entry| entry.sources().contains(&SourceLabel::Ccd)));
    Ok(())
}

#[test]
fn nces_the_ccd_reader_maps_identity_address_grades_and_charter_status() -> TestResult {
    let outcome = parse_ccd(CCD)?;

    let middle = entry(&outcome, "nces:010000500870")?;
    check!(eq;
        middle.name().map(|name| name.as_str()),
        Some("Albertville Middle School")
    );
    let address = middle.address().ok_or("missing middle school address")?;
    check!(eq;
        address.line1().map(|line| line.as_str()),
        Some("600 E Alabama Ave")
    );
    check!(eq;
        address.city().map(|city| city.as_str()),
        Some("Albertville")
    );
    check!(eq; address.state(), Some(UsJurisdiction::Alabama));
    check!(eq;
        address.zip().map(|zip| zip.to_string()),
        Some("35950-2336".to_string())
    );
    check!(eq;
        middle.phone().map(|phone| phone.as_str()),
        Some("(256)878-2341")
    );
    check!(eq;
        middle.website().map(|website| website.as_str()),
        Some("http://www.albertk12.org")
    );
    check!(eq; middle.grades().ok_or("missing middle grades")?.label(), "7-8");
    check!(eq; middle.kind(), Some(&SchoolKind::public()));

    let high = entry(&outcome, "nces:010000500871")?;
    check!(eq;
        high.name().map(|name| name.as_str()),
        Some("Albertville High School")
    );
    check!(eq; high.grades().ok_or("missing high grades")?.label(), "9-12");
    check!(eq;
        high.address()
            .and_then(|address| address.zip())
            .map(|zip| zip.to_string()),
        Some("35950-2322".to_string())
    );

    let missing_four = entry(&outcome, "nces:010222001513")?;
    check!(eq;
        missing_four
            .address()
            .and_then(|address| address.zip())
            .map(|zip| zip.to_string()),
        Some("35811".to_string())
    );
    check!(eq; missing_four.grades().ok_or("missing elementary grades")?.label(), "PK-3");

    let ungraded = entry(&outcome, "nces:010000602755")?;
    check!(eq; ungraded.grades(), None);
    check!(eq;
        ungraded
            .address()
            .and_then(|address| address.zip())
            .map(|zip| zip.to_string()),
        Some("35951".to_string())
    );

    let charter = entry(&outcome, "nces:010019702432")?;
    check!(eq; charter.kind(), Some(&SchoolKind::charter()));
    check!(eq;
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
    check!(eq; charters, 28);
    check!(outcome
        .entries()
        .iter()
        .all(|entry| matches!(entry.key(), DirectoryKey::Nces(_))));
    Ok(())
}

#[test]
fn nces_the_ccd_reader_refuses_a_file_without_the_identity_column() -> TestResult {
    let mutated = CCD.replacen("NCESSCH", "SCHOOL_CODE", 1);
    let detail = match parse_ccd(&mutated) {
        Err(CrawlError::Invariant { detail }) => detail,
        Err(error) => return Err(format!("expected invariant refusal, got {error}").into()),
        Ok(_) => return Err("missing identity column accepted".into()),
    };
    check!(detail.contains("NCESSCH"), "{detail}");
    Ok(())
}

#[test]
fn nces_the_pss_window_maps_names_addresses_coordinates_and_enrollment() -> TestResult {
    let outcome = parse_pss(PSS)?;
    check!(eq;
        outcome.counts(),
        ReadCounts {
            entries: 374,
            skipped: 25,
            notes: 0
        }
    );
    check!(outcome
        .skipped()
        .iter()
        .all(|issue| issue.field == "state" && issue.detail.contains("AK")));
    check!(outcome
        .entries()
        .iter()
        .all(|entry| entry.sources().contains(&SourceLabel::Pss)));

    let pilgrim = entry(&outcome, "pss:A2380006")?;
    check!(eq;
        pilgrim.name().map(|name| name.as_str()),
        Some("MT. PILGRIM CHRISTIAN ACADEMY")
    );
    let address = pilgrim.address().ok_or("missing pilgrim address")?;
    check!(eq;
        address.line1().map(|line| line.as_str()),
        Some("6746 Grasselli Rd")
    );
    check!(eq; address.city().map(|city| city.as_str()), Some("Fairfield"));
    check!(eq; address.state(), Some(UsJurisdiction::Alabama));
    check!(eq;
        address.zip().map(|zip| zip.to_string()),
        Some("35064".to_string())
    );
    check!(eq;
        pilgrim.phone().map(|phone| phone.as_str()),
        Some("2057805096")
    );
    check!(eq; pilgrim.enrollment().map(|count| count.get()), Some(49));
    check!(eq; pilgrim.kind(), Some(&SchoolKind::private()));
    let coordinates = pilgrim.coordinates().ok_or("missing pilgrim coordinates")?;
    check!(eq; coordinates.latitude().to_string(), "33.46998");
    check!(eq; coordinates.longitude().to_string(), "-86.924519");
    Ok(())
}

#[test]
fn nces_a_quoted_pss_field_keeps_the_comma_it_carries() -> TestResult {
    let outcome = parse_pss(PSS)?;

    let quoted_name = entry(&outcome, "pss:A1970033")?;
    check!(eq;
        quoted_name.name().map(|name| name.as_str()),
        Some("COTTAGE HILL CHRISTIAN ACADEMY, LOWER")
    );

    let quoted_street = entry(&outcome, "pss:A0100219")?;
    check!(eq;
        quoted_street
            .address()
            .and_then(|address| address.line1())
            .map(|line| line.as_str()),
        Some("130 N St E., Ste C")
    );
    Ok(())
}

#[test]
fn nces_the_pss_reader_refuses_a_file_without_the_name_column() -> TestResult {
    let mutated = PSS.replacen("PINST", "SCHOOL_NAME", 1);
    let detail = match parse_pss(&mutated) {
        Err(CrawlError::Invariant { detail }) => detail,
        Err(error) => return Err(format!("expected invariant refusal, got {error}").into()),
        Ok(_) => return Err("missing name column accepted".into()),
    };
    check!(detail.contains("PINST"), "{detail}");
    Ok(())
}

#[test]
fn nces_an_empty_artifact_is_refused_rather_than_read_as_no_rows() {
    assert!(matches!(parse_ccd(""), Err(CrawlError::Invariant { .. })));
    assert!(matches!(parse_pss(""), Err(CrawlError::Invariant { .. })));
}
