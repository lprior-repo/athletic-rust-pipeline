use super::parse_listing;
use crate::directory::ReadOutcome;
use crate::CrawlError;
use census_domain::school_directory::{
    CityName, DirectoryKey, SchoolDirectoryEntry, SchoolName, SourceLabel, StreetLine,
};
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const NAIS_LISTING: &str = r#"<!doctype html>
<html>
<head><title>NAIS Member School Directory</title></head>
<body>
<h1>National Association of Independent Schools</h1>
<div class="school-list-item">
<h3>Riverside Academy</h3>
<div class="address">100 River Road<br>Springfield, IL 62704</div>
</div>
<div class="school-list-item">
<h3>Lakeside Country Day &amp; School</h3>
<div class="address">55 Lake Street, Springfield, IL 62704-1234</div>
</div>
</body>
</html>
"#;

const NAIS_LISTING_WITH_NAMELESS_ROW: &str = r#"<html>
<body>
<h1>NAIS</h1>
<div class="school-list-item">
<h3>Riverside Academy</h3>
<div class="address">100 River Road, Springfield, IL 62704</div>
</div>
<div class="school-list-item">
<div class="address">9 Nowhere Road</div>
</div>
</body>
</html>
"#;

const NAIS_PAGE_WITHOUT_LISTING: &str = r#"<html>
<head><title>NAIS</title></head>
<body><p>Member sign-in</p></body>
</html>
"#;

const RENAISSANCE_LISTING: &str = r#"<html>
<body>
<div class="school-list-item">
<h3>Renaissance Academy</h3>
</div>
</body>
</html>
"#;

const CAPE_LISTING: &str = r#"<html>
<head><title>CAPE Member Schools</title></head>
<body>
<div class="school-list-item">
<h3>Harbor Day School</h3>
<div class="address">Springfield, IL</div>
</div>
</body>
</html>
"#;

const NASSP_LISTING: &str = r#"<html>
<head><title>National Association of Secondary School Principals</title></head>
<body>
<div class="school-list-item">
<h3>Northgate High School</h3>
<div class="address">12 Northgate Way, Helena, MT 59601</div>
</div>
</body>
</html>
"#;

const NAIS_LISTING_WITH_BAD_ZIP: &str = r#"<body>
<h1>NAIS</h1>
<div class="school-list-item">
<h3>Riverside Academy</h3>
<div class="address">100 River Road, Springfield, IL 6270</div>
</div>
</body>
"#;

const NAIS_LISTING_WITH_LONG_CITY: &str = r#"<body>
<h1>NAIS</h1>
<div class="school-list-item">
<h3>Riverside Academy</h3>
<div class="address">1 Long Road, Springfield Springfield Springfield Springfield Springfield SPRINGFIELD, IL 62704</div>
</div>
</body>
"#;

const NAIS_LISTING_IN_HAWAII: &str = r#"<body>
<h1>NAIS</h1>
<div class="school-list-item">
<h3>Pacific Academy</h3>
<div class="address">Springfield, HI 96813</div>
</div>
</body>
"#;

fn first(outcome: &ReadOutcome) -> TestResult<&SchoolDirectoryEntry> {
    Ok(outcome.entries().first().ok_or("one entry")?)
}

fn source_label(entry: &SchoolDirectoryEntry) -> Vec<String> {
    entry.sources().iter().map(SourceLabel::label).collect()
}

#[test]
fn a_listing_yields_one_weak_entry_per_item() -> TestResult {
    let outcome = parse_listing(NAIS_LISTING)?;
    check!(eq; outcome.counts().entries, 2);
    check!(eq; outcome.counts().skipped, 0);
    check!(eq; outcome.counts().notes, 0);

    let riverside = first(&outcome)?;
    check!(eq;
        riverside.name().map(SchoolName::as_str),
        Some("Riverside Academy")
    );
    check!(matches!(riverside.key(), DirectoryKey::Weak(_)));
    check!(eq;
        riverside.key().label(),
        "weak:riverside academy|springfield|IL"
    );
    check!(eq;
        source_label(riverside),
        vec!["association:NAIS".to_string()]
    );
    let address = riverside.address().ok_or("street address")?;
    check!(eq;
        address.line1().map(StreetLine::as_str),
        Some("100 River Road")
    );
    check!(eq; address.city().map(CityName::as_str), Some("Springfield"));
    check!(eq; address.state(), Some(UsJurisdiction::Illinois));
    check!(eq;
        address.zip().map(|zip| zip.to_string()),
        Some("62704".to_string())
    );

    let lakeside = outcome.entries().get(1).ok_or("second entry")?;
    check!(eq;
        lakeside.name().map(SchoolName::as_str),
        Some("Lakeside Country Day & School")
    );
    let address = lakeside.address().ok_or("second address")?;
    check!(eq;
        address.line1().map(StreetLine::as_str),
        Some("55 Lake Street")
    );
    check!(eq; address.city().map(CityName::as_str), Some("Springfield"));
    check!(eq;
        address.zip().map(|zip| zip.to_string()),
        Some("62704-1234".to_string())
    );
    Ok(())
}

#[test]
fn a_row_without_a_usable_name_lands_in_the_ledger() -> TestResult {
    let outcome = parse_listing(NAIS_LISTING_WITH_NAMELESS_ROW)?;
    check!(eq; outcome.counts().entries, 1);
    check!(eq; outcome.counts().skipped, 1);
    let issue = outcome.skipped().first().ok_or("one skip")?;
    check!(eq; issue.line, 8);
    check!(eq; issue.field, "name");
    check!(eq;
        first(&outcome)?.name().map(SchoolName::as_str),
        Some("Riverside Academy")
    );
    Ok(())
}

#[test]
fn a_name_that_leaves_no_matching_form_lands_in_the_ledger() -> TestResult {
    let body = NAIS_LISTING.replace("Riverside Academy", "---");
    let outcome = parse_listing(&body)?;
    check!(eq; outcome.counts().entries, 1);
    check!(eq; outcome.counts().skipped, 1);
    let issue = outcome.skipped().first().ok_or("one skip")?;
    check!(eq; issue.field, "name");
    Ok(())
}

#[test]
fn a_body_with_no_listing_is_refused() -> TestResult {
    let error = match parse_listing(NAIS_PAGE_WITHOUT_LISTING) {
        Err(error) => error,
        Ok(_) => return Err("there is no listing".into()),
    };
    check!(matches!(error, CrawlError::Invariant { .. }));
    Ok(())
}

#[test]
fn a_body_that_names_no_association_is_refused() -> TestResult {
    let error = match parse_listing(RENAISSANCE_LISTING) {
        Err(error) => error,
        Ok(_) => return Err("no association is named".into()),
    };
    check!(matches!(&error, CrawlError::Invariant { .. }));
    Ok(())
}

#[test]
fn the_association_name_comes_from_the_body() -> TestResult {
    let nais = parse_listing(NAIS_LISTING)?;
    check!(eq;
        source_label(first(&nais)?),
        vec!["association:NAIS".to_string()]
    );

    let cape = parse_listing(CAPE_LISTING)?;
    check!(eq;
        source_label(first(&cape)?),
        vec!["association:CAPE".to_string()]
    );
    let harbor = first(&cape)?;
    check!(eq;
        harbor.key().label(),
        "weak:harbor day school|springfield|IL"
    );
    let address = harbor.address().ok_or("city and state")?;
    check!(eq; address.line1(), None);
    check!(eq; address.city().map(CityName::as_str), Some("Springfield"));
    check!(eq; address.zip(), None);

    let nassp = parse_listing(NASSP_LISTING)?;
    check!(eq;
        source_label(first(&nassp)?),
        vec!["association:NASSP".to_string()]
    );
    Ok(())
}

#[test]
fn a_published_zip_that_is_not_a_zip_is_a_note_and_the_row_survives() -> TestResult {
    let outcome = parse_listing(NAIS_LISTING_WITH_BAD_ZIP)?;
    check!(eq; outcome.counts().entries, 1);
    check!(eq; outcome.counts().skipped, 0);
    let note = outcome.notes().first().ok_or("one note")?;
    check!(eq; note.field, "zip");
    check!(note.detail.contains("6270"), "{}", note.detail);
    let entry = first(&outcome)?;
    let address = entry.address().ok_or("street address")?;
    check!(eq;
        address.line1().map(StreetLine::as_str),
        Some("100 River Road")
    );
    check!(eq; address.city().map(CityName::as_str), Some("Springfield"));
    check!(eq; address.state(), Some(UsJurisdiction::Illinois));
    check!(eq; address.zip(), None);
    Ok(())
}

#[test]
fn a_city_too_long_for_the_domain_is_a_note_and_the_row_survives() -> TestResult {
    let outcome = parse_listing(NAIS_LISTING_WITH_LONG_CITY)?;
    check!(eq; outcome.counts().entries, 1);
    check!(eq; outcome.counts().skipped, 0);
    let note = outcome.notes().first().ok_or("one note")?;
    check!(eq; note.field, "city");
    let address = first(&outcome)?.address().ok_or("street address")?;
    check!(eq; address.city(), None);
    check!(eq; address.state(), Some(UsJurisdiction::Illinois));
    check!(eq;
        address.zip().map(|zip| zip.to_string()),
        Some("62704".to_string())
    );
    Ok(())
}

#[test]
fn a_state_outside_the_census_scope_is_not_claimed() -> TestResult {
    let outcome = parse_listing(NAIS_LISTING_IN_HAWAII)?;
    let entry = first(&outcome)?;
    check!(eq; entry.key().label(), "weak:pacific academy|springfield|");
    let address = entry.address().ok_or("city and zip")?;
    check!(eq; address.state(), None);
    check!(eq; address.city().map(CityName::as_str), Some("Springfield"));
    check!(eq;
        address.zip().map(|zip| zip.to_string()),
        Some("96813".to_string())
    );
    Ok(())
}

#[test]
fn an_item_without_an_address_still_makes_a_weak_entry() -> TestResult {
    let body = NAIS_LISTING.replace("100 River Road<br>Springfield, IL 62704", "");
    let outcome = parse_listing(&body)?;
    check!(eq; outcome.counts().entries, 2);
    let entry = first(&outcome)?;
    check!(eq; entry.key().label(), "weak:riverside academy||");
    check!(eq; entry.address(), None);
    Ok(())
}
