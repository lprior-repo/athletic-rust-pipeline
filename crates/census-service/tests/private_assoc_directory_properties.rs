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
fn private_assoc_listing_yields_one_weak_entry_per_row() -> Result<(), Box<dyn std::error::Error>> {
    let outcome = parse_listing(LISTING)?;
    let left = [
        outcome.counts().entries,
        outcome.counts().skipped,
        outcome.counts().notes,
    ];
    let right = [2, 0, 0];
    if left != right {
        return Err(format!("entries/skipped/notes: left={left:?}, right={right:?}").into());
    }

    let riverside = outcome.entries().first().ok_or("missing first entry")?;
    if !matches!(riverside.key(), DirectoryKey::Weak(_)) {
        return Err(format!("expected weak key, got {:?}", riverside.key()).into());
    }
    let name = riverside.name().map(SchoolName::as_str);
    if name != Some("Riverside Academy") {
        return Err(format!("name: left={name:?}, right={:?}", Some("Riverside Academy")).into());
    }
    let key = riverside.key().label();
    if key != "weak:riverside academy|springfield|IL" {
        return Err(
            format!("key: left={key:?}, right=\"weak:riverside academy|springfield|IL\"").into(),
        );
    }
    let sources = riverside
        .sources()
        .iter()
        .map(SourceLabel::label)
        .collect::<Vec<_>>();
    let expected_sources = vec!["association:NAIS".to_string()];
    if sources != expected_sources {
        return Err(format!("sources: left={sources:?}, right={expected_sources:?}").into());
    }
    let address = riverside.address().ok_or("missing first address")?;
    let state = address.state();
    if state != Some(UsJurisdiction::Illinois) {
        return Err(format!(
            "state: left={state:?}, right={:?}",
            Some(UsJurisdiction::Illinois)
        )
        .into());
    }
    let zip = address.zip().map(|zip| zip.to_string());
    let expected_zip = Some("62704".to_string());
    if zip != expected_zip {
        return Err(format!("zip: left={zip:?}, right={expected_zip:?}").into());
    }

    let lakeside = outcome.entries().get(1).ok_or("missing second entry")?;
    let address = lakeside.address().ok_or("missing second address")?;
    let zip = address.zip().map(|zip| zip.to_string());
    let expected_zip = Some("62704-1234".to_string());
    if zip != expected_zip {
        return Err(format!("zip: left={zip:?}, right={expected_zip:?}").into());
    }
    Ok(())
}

#[test]
fn private_assoc_row_without_a_name_lands_in_the_ledger() -> Result<(), Box<dyn std::error::Error>>
{
    let outcome = parse_listing(NAMELESS_ROW)?;
    let left = [outcome.counts().entries, outcome.counts().skipped];
    let right = [1, 1];
    if left != right {
        return Err(format!("entries/skipped: left={left:?}, right={right:?}").into());
    }
    let issue = outcome.skipped().first().ok_or("missing skipped row")?;
    if issue.line != 8 {
        return Err(format!("issue line: left={}, right=8", issue.line).into());
    }
    if issue.field != "name" {
        return Err(format!("issue field: left={:?}, right=\"name\"", issue.field).into());
    }
    let rendered = issue.render();
    if rendered != "line 8: name school name is empty" {
        return Err(format!(
            "issue rendering: left={rendered:?}, right=\"line 8: name school name is empty\""
        )
        .into());
    }
    Ok(())
}

#[test]
fn private_assoc_body_without_a_listing_is_refused() {
    assert!(matches!(
        parse_listing(NO_LISTING),
        Err(CrawlError::Invariant { .. })
    ));
}

#[test]
fn private_assoc_body_with_no_association_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let error = match parse_listing(NO_ASSOCIATION) {
        Err(error) => error,
        Ok(_) => return Err("body without an association accepted".into()),
    };
    if !matches!(&error, CrawlError::Invariant { .. }) {
        return Err(format!("expected invariant refusal, got {error:?}").into());
    }
    let rendered = error.to_string();
    if !rendered.contains("NAIS") {
        return Err(format!("refusal omitted NAIS: {rendered}").into());
    }
    if !rendered.contains("CAPE") {
        return Err(format!("refusal omitted CAPE: {rendered}").into());
    }
    if !rendered.contains("NASSP") {
        return Err(format!("refusal omitted NASSP: {rendered}").into());
    }
    Ok(())
}
