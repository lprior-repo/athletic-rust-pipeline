use super::parse_listing;
use crate::directory::ReadOutcome;
use crate::CrawlError;
use census_domain::school_directory::{
    AssociationLabel, CityName, DirectoryKey, SchoolDirectoryEntry, SchoolName, SourceLabel,
    StreetLine,
};
use census_domain::UsJurisdiction;

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

fn first(outcome: &ReadOutcome) -> &SchoolDirectoryEntry {
    outcome.entries().first().expect("one entry")
}

fn source_label(entry: &SchoolDirectoryEntry) -> Vec<String> {
    entry.sources().iter().map(SourceLabel::label).collect()
}

fn nais_label() -> SourceLabel {
    SourceLabel::PrivateAssociation {
        label: AssociationLabel::parse("NAIS").expect("NAIS label"),
    }
}

#[test]
fn a_listing_yields_one_weak_entry_per_item() {
    let outcome = parse_listing(NAIS_LISTING).expect("the NAIS listing reads");
    assert_eq!(outcome.counts().entries, 2);
    assert_eq!(outcome.counts().skipped, 0);
    assert_eq!(outcome.counts().notes, 0);

    let riverside = first(&outcome);
    assert_eq!(
        riverside.name().map(SchoolName::as_str),
        Some("Riverside Academy")
    );
    assert!(matches!(riverside.key(), DirectoryKey::Weak(_)));
    assert_eq!(
        riverside.key().label(),
        "weak:riverside academy|springfield|IL"
    );
    assert_eq!(
        source_label(riverside),
        vec!["association:NAIS".to_string()]
    );
    assert_eq!(
        riverside.sources(),
        &std::collections::BTreeSet::from([nais_label()])
    );
    let address = riverside.address().expect("street address");
    assert_eq!(
        address.line1().map(StreetLine::as_str),
        Some("100 River Road")
    );
    assert_eq!(address.city().map(CityName::as_str), Some("Springfield"));
    assert_eq!(address.state(), Some(UsJurisdiction::Illinois));
    assert_eq!(
        address.zip().map(|zip| zip.to_string()),
        Some("62704".to_string())
    );

    let lakeside = outcome.entries().get(1).expect("second entry");
    assert_eq!(
        lakeside.name().map(SchoolName::as_str),
        Some("Lakeside Country Day & School")
    );
    let address = lakeside.address().expect("second address");
    assert_eq!(
        address.line1().map(StreetLine::as_str),
        Some("55 Lake Street")
    );
    assert_eq!(address.city().map(CityName::as_str), Some("Springfield"));
    assert_eq!(
        address.zip().map(|zip| zip.to_string()),
        Some("62704-1234".to_string())
    );
}

#[test]
fn a_row_without_a_usable_name_lands_in_the_ledger() {
    let outcome = parse_listing(NAIS_LISTING_WITH_NAMELESS_ROW).expect("the listing reads");
    assert_eq!(outcome.counts().entries, 1);
    assert_eq!(outcome.counts().skipped, 1);
    let issue = outcome.skipped().first().expect("one skip");
    assert_eq!(issue.line, 8);
    assert_eq!(issue.field, "name");
    assert_eq!(issue.render(), "line 8: name school name is empty");
    assert_eq!(
        first(&outcome).name().map(SchoolName::as_str),
        Some("Riverside Academy")
    );
}

#[test]
fn a_name_that_leaves_no_matching_form_lands_in_the_ledger() {
    let body = NAIS_LISTING.replace("Riverside Academy", "---");
    let outcome = parse_listing(&body).expect("the listing reads");
    assert_eq!(outcome.counts().entries, 1);
    assert_eq!(outcome.counts().skipped, 1);
    let issue = outcome.skipped().first().expect("one skip");
    assert_eq!(issue.field, "name");
    assert_eq!(issue.detail, "matching name is empty");
}

#[test]
fn a_body_with_no_listing_is_refused() {
    let error = parse_listing(NAIS_PAGE_WITHOUT_LISTING).expect_err("there is no listing");
    assert!(matches!(error, CrawlError::Invariant { .. }));
}

#[test]
fn a_body_that_names_no_association_is_refused() {
    let error = parse_listing(RENAISSANCE_LISTING).expect_err("no association is named");
    assert!(matches!(&error, CrawlError::Invariant { .. }));
    let rendered = error.to_string();
    assert!(rendered.contains("NAIS"), "{rendered}");
    assert!(rendered.contains("CAPE"), "{rendered}");
    assert!(rendered.contains("NASSP"), "{rendered}");
}

#[test]
fn the_association_name_comes_from_the_body() {
    let nais = parse_listing(NAIS_LISTING).expect("the NAIS listing reads");
    assert_eq!(
        source_label(first(&nais)),
        vec!["association:NAIS".to_string()]
    );

    let cape = parse_listing(CAPE_LISTING).expect("the CAPE listing reads");
    assert_eq!(
        source_label(first(&cape)),
        vec!["association:CAPE".to_string()]
    );
    let harbor = first(&cape);
    assert_eq!(
        harbor.key().label(),
        "weak:harbor day school|springfield|IL"
    );
    let address = harbor.address().expect("city and state");
    assert_eq!(address.line1(), None);
    assert_eq!(address.city().map(CityName::as_str), Some("Springfield"));
    assert_eq!(address.zip(), None);

    let nassp = parse_listing(NASSP_LISTING).expect("the NASSP listing reads");
    assert_eq!(
        source_label(first(&nassp)),
        vec!["association:NASSP".to_string()]
    );
}

#[test]
fn a_published_zip_that_is_not_a_zip_is_a_note_and_the_row_survives() {
    let outcome = parse_listing(NAIS_LISTING_WITH_BAD_ZIP).expect("the listing reads");
    assert_eq!(outcome.counts().entries, 1);
    assert_eq!(outcome.counts().skipped, 0);
    let note = outcome.notes().first().expect("one note");
    assert_eq!(note.field, "zip");
    assert!(note.detail.contains("6270"), "{}", note.detail);
    let entry = first(&outcome);
    let address = entry.address().expect("street address");
    assert_eq!(
        address.line1().map(StreetLine::as_str),
        Some("100 River Road")
    );
    assert_eq!(address.city().map(CityName::as_str), Some("Springfield"));
    assert_eq!(address.state(), Some(UsJurisdiction::Illinois));
    assert_eq!(address.zip(), None);
}

#[test]
fn a_city_too_long_for_the_domain_is_a_note_and_the_row_survives() {
    let outcome = parse_listing(NAIS_LISTING_WITH_LONG_CITY).expect("the listing reads");
    assert_eq!(outcome.counts().entries, 1);
    assert_eq!(outcome.counts().skipped, 0);
    let note = outcome.notes().first().expect("one note");
    assert_eq!(note.field, "city");
    let address = first(&outcome).address().expect("street address");
    assert_eq!(address.city(), None);
    assert_eq!(address.state(), Some(UsJurisdiction::Illinois));
    assert_eq!(
        address.zip().map(|zip| zip.to_string()),
        Some("62704".to_string())
    );
}

#[test]
fn a_state_outside_the_census_scope_is_not_claimed() {
    let outcome = parse_listing(NAIS_LISTING_IN_HAWAII).expect("the listing reads");
    let entry = first(&outcome);
    assert_eq!(entry.key().label(), "weak:pacific academy|springfield|");
    let address = entry.address().expect("city and zip");
    assert_eq!(address.state(), None);
    assert_eq!(address.city().map(CityName::as_str), Some("Springfield"));
    assert_eq!(
        address.zip().map(|zip| zip.to_string()),
        Some("96813".to_string())
    );
}

#[test]
fn an_item_without_an_address_still_makes_a_weak_entry() {
    let body = NAIS_LISTING.replace("100 River Road<br>Springfield, IL 62704", "");
    let outcome = parse_listing(&body).expect("the listing reads");
    assert_eq!(outcome.counts().entries, 2);
    let entry = first(&outcome);
    assert_eq!(entry.key().label(), "weak:riverside academy||");
    assert_eq!(entry.address(), None);
}
