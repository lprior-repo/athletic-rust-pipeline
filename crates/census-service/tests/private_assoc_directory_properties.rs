use census_crawl::private_assoc::parse_listing;
use census_crawl::CrawlError;
use census_domain::school_directory::{DirectoryKey, SchoolName, SourceLabel};
use census_domain::UsJurisdiction;

const LISTING: &str = r#"<html>
<head><title>NAIS Member School Directory</title></head>
<body>
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

const NAMELESS_ROW: &str = r#"<html>
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

const NO_LISTING: &str = r#"<html>
<head><title>NAIS</title></head>
<body><p>Member sign-in</p></body>
</html>
"#;

const NO_ASSOCIATION: &str = r#"<html>
<body>
<div class="school-list-item">
<h3>Renaissance Academy</h3>
</div>
</body>
</html>
"#;

#[test]
fn private_assoc_listing_yields_one_weak_entry_per_row() {
    let outcome = parse_listing(LISTING).expect("the listing reads");
    assert_eq!(outcome.counts().entries, 2);
    assert_eq!(outcome.counts().skipped, 0);
    assert_eq!(outcome.counts().notes, 0);

    let riverside = outcome.entries().first().expect("first entry");
    assert!(matches!(riverside.key(), DirectoryKey::Weak(_)));
    assert_eq!(
        riverside.name().map(SchoolName::as_str),
        Some("Riverside Academy")
    );
    assert_eq!(
        riverside.key().label(),
        "weak:riverside academy|springfield|IL"
    );
    assert_eq!(
        riverside
            .sources()
            .iter()
            .map(SourceLabel::label)
            .collect::<Vec<_>>(),
        vec!["association:NAIS".to_string()]
    );
    let address = riverside.address().expect("first address");
    assert_eq!(address.state(), Some(UsJurisdiction::Illinois));
    assert_eq!(
        address.zip().map(|zip| zip.to_string()),
        Some("62704".to_string())
    );

    let lakeside = outcome.entries().get(1).expect("second entry");
    let address = lakeside.address().expect("second address");
    assert_eq!(
        address.zip().map(|zip| zip.to_string()),
        Some("62704-1234".to_string())
    );
}

#[test]
fn private_assoc_row_without_a_name_lands_in_the_ledger() {
    let outcome = parse_listing(NAMELESS_ROW).expect("the listing reads");
    assert_eq!(outcome.counts().entries, 1);
    assert_eq!(outcome.counts().skipped, 1);
    let issue = outcome.skipped().first().expect("one skip");
    assert_eq!(issue.line, 8);
    assert_eq!(issue.field, "name");
    assert_eq!(issue.render(), "line 8: name school name is empty");
}

#[test]
fn private_assoc_body_without_a_listing_is_refused() {
    let error = parse_listing(NO_LISTING).expect_err("the body carries no listing");
    assert!(matches!(error, CrawlError::Invariant { .. }));
}

#[test]
fn private_assoc_body_with_no_association_is_refused() {
    let error = parse_listing(NO_ASSOCIATION).expect_err("the body names no association");
    assert!(matches!(&error, CrawlError::Invariant { .. }));
    let rendered = error.to_string();
    assert!(rendered.contains("NAIS"), "{rendered}");
    assert!(rendered.contains("CAPE"), "{rendered}");
    assert!(rendered.contains("NASSP"), "{rendered}");
}
