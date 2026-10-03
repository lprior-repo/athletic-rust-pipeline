use census_crawl::state_ed::{parse_index, parse_profile, parse_tabular};
use census_domain::school_directory::{
    CityName, DirectoryKey, Enrollment, Phone, SchoolName, StreetLine, Website,
};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const INDEX_A: &str =
    include_str!("../../census-crawl/tests/fixtures/state_ed/index_letter_a.html");
const KINGSTON: &str =
    include_str!("../../census-crawl/tests/fixtures/state_ed/profile_kingston.html");

#[test]
fn state_ed_index_lists_the_letter_a_schools() -> TestResult {
    let outcome = parse_index(INDEX_A)?;
    let left = [outcome.entries().len(), outcome.counts().skipped];
    let right = [220, 0];
    if left != right {
        return Err(format!(
            "entries/skipped (every row carries an id and a name): left={left:?}, right={right:?}"
        )
        .into());
    }
    Ok(())
}

#[test]
fn state_ed_index_contains_kingston_middle() -> TestResult {
    let outcome = parse_index(INDEX_A)?;
    let names: Vec<&str> = outcome
        .entries()
        .iter()
        .filter_map(|entry| entry.name().map(SchoolName::as_str))
        .collect();
    if !names.contains(&"A A KINGSTON MIDDLE SCHOOL") {
        return Err(format!(
            "index should list A A KINGSTON MIDDLE SCHOOL; found {} schools",
            names.len()
        )
        .into());
    }
    Ok(())
}

#[test]
fn state_ed_profile_kingston_extracts_identity() -> TestResult {
    let outcome = parse_profile(KINGSTON)?;
    if outcome.entries().len() != 1 {
        return Err(format!("entry count: left={}, right=1", outcome.entries().len()).into());
    }
    let entry = outcome.entries().first().ok_or("missing entry")?;
    let name = entry.name().map(SchoolName::as_str);
    if name != Some("A A KINGSTON MIDDLE SCHOOL") {
        return Err(format!(
            "name: left={name:?}, right={:?}",
            Some("A A KINGSTON MIDDLE SCHOOL")
        )
        .into());
    }
    match entry.key() {
        DirectoryKey::StateRecord { state, id } => {
            if state != &UsJurisdiction::NewYork {
                return Err(
                    format!("state: left={state:?}, right={:?}", UsJurisdiction::NewYork).into(),
                );
            }
            let record_id = id.as_str();
            if record_id != "800000038718" {
                return Err(
                    format!("record id: left={record_id:?}, right=\"800000038718\"").into(),
                );
            }
        }
        other => return Err(format!("expected StateRecord key, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn state_ed_profile_kingston_extracts_contact_and_size() -> TestResult {
    let outcome = parse_profile(KINGSTON)?;
    let entry = outcome.entries().first().ok_or("missing entry")?;
    let phone = entry.phone().map(Phone::as_str);
    if phone != Some("3152652000") {
        return Err(format!(
            "phone holds the digits of the tel: href: left={phone:?}, right={:?}",
            Some("3152652000")
        )
        .into());
    }
    let website = entry.website().map(Website::as_str);
    if website != Some("https://www.potsdamcsd.org") {
        return Err(format!(
            "website: left={website:?}, right={:?}",
            Some("https://www.potsdamcsd.org")
        )
        .into());
    }
    let enrollment = entry.enrollment().map(Enrollment::get);
    if enrollment != Some(393) {
        return Err(format!("enrollment: left={enrollment:?}, right=Some(393)").into());
    }
    Ok(())
}

#[test]
fn state_ed_profile_kingston_extracts_maps_address() -> TestResult {
    let outcome = parse_profile(KINGSTON)?;
    let entry = outcome.entries().first().ok_or("missing entry")?;
    let address = entry.address().ok_or("missing profile address")?;
    let street = address.line1().map(StreetLine::as_str);
    if street != Some("29 Leroy St") {
        return Err(format!("street: left={street:?}, right={:?}", Some("29 Leroy St")).into());
    }
    let city = address.city().map(CityName::as_str);
    if city != Some("Potsdam") {
        return Err(format!("city: left={city:?}, right={:?}", Some("Potsdam")).into());
    }
    let state = address.state();
    if state != Some(UsJurisdiction::NewYork) {
        return Err(format!(
            "state: left={state:?}, right={:?}",
            Some(UsJurisdiction::NewYork)
        )
        .into());
    }
    let zip = address.zip().map(|zip| zip.code());
    if zip != Some("13676") {
        return Err(format!("zip: left={zip:?}, right={:?}", Some("13676")).into());
    }
    Ok(())
}

#[test]
fn state_ed_profile_kingston_source_is_nysed() -> TestResult {
    let outcome = parse_profile(KINGSTON)?;
    let entry = outcome.entries().first().ok_or("missing entry")?;
    let labels: Vec<String> = entry.sources().iter().map(|label| label.label()).collect();
    if !labels.contains(&"state-ed:NY".to_string()) {
        return Err(format!("missing state-ed:NY source: {labels:?}").into());
    }
    Ok(())
}

#[test]
fn state_ed_profile_refuses_a_page_that_is_not_a_profile() -> TestResult {
    let error = match parse_profile("<html><body><p>Nothing here</p></body></html>") {
        Err(error) => error,
        Ok(_) => return Err("non-profile page accepted".into()),
    };
    if !matches!(&error, census_crawl::CrawlError::DirectoryArtifact { .. }) {
        return Err(format!("the refusal names the artifact: {error}").into());
    }
    Ok(())
}

#[test]
fn state_ed_index_reads_no_rows_from_a_profile() -> TestResult {
    let outcome = parse_index(KINGSTON)?;
    let entries = outcome.entries().len();
    if entries != 0 {
        return Err(format!("entry count: left={entries}, right=0").into());
    }
    Ok(())
}

#[test]
fn state_ed_profile_is_idempotent() -> TestResult {
    let first = parse_profile(KINGSTON)?;
    let second = parse_profile(KINGSTON)?;
    if first != second {
        return Err(format!("profile replay: left={first:?}, right={second:?}").into());
    }
    Ok(())
}

#[test]
fn state_ed_index_is_idempotent() -> TestResult {
    let first = parse_index(INDEX_A)?;
    let second = parse_index(INDEX_A)?;
    if first != second {
        return Err(format!("index replay: left={first:?}, right={second:?}").into());
    }
    Ok(())
}

#[test]
fn state_ed_tabular_reads_documented_columns() -> TestResult {
    let text = "NAME,CITY,STATE,STREET,ZIP,PHONE\n\
                A A Kingston Middle School,Potsdam,NY,29 Leroy St,13676,315-265-2000\n";
    let outcome = parse_tabular(text)?;
    let entry = outcome.entries().first().ok_or("missing row")?;
    let name = entry.name().map(SchoolName::as_str);
    if name != Some("A A Kingston Middle School") {
        return Err(format!(
            "name: left={name:?}, right={:?}",
            Some("A A Kingston Middle School")
        )
        .into());
    }
    let address = entry.address().ok_or("missing row address")?;
    let city = address.city().map(CityName::as_str);
    if city != Some("Potsdam") {
        return Err(format!("city: left={city:?}, right={:?}", Some("Potsdam")).into());
    }
    let state = address.state();
    if state != Some(UsJurisdiction::NewYork) {
        return Err(format!(
            "state: left={state:?}, right={:?}",
            Some(UsJurisdiction::NewYork)
        )
        .into());
    }
    Ok(())
}

#[test]
fn state_ed_tabular_refuses_an_unknown_shape() -> TestResult {
    let error = match parse_tabular("school,city,state\nFoo,Bar,NY\n") {
        Err(error) => error,
        Ok(_) => return Err("unknown tabular shape accepted".into()),
    };
    match error {
        census_crawl::CrawlError::Invariant { detail } => {
            if !detail.contains("NAME") {
                return Err(format!("the refusal names the column: {detail}").into());
            }
        }
        other => return Err(format!("expected a shape refusal, got {other}").into()),
    }
    Ok(())
}
