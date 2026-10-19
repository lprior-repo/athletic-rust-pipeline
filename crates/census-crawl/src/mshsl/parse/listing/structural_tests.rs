use crate::mshsl::parse_school_list;
use crate::CrawlError;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ROW: &str = "<div class=\"views-row\">";
const VALID: &str = "<div class=\"views-row\"><div class=\"school-teaser\"><a href=\"/schools/controlled-alpha\" class=\"school-teaser__title\">Controlled Alpha</a><span class=\"locality\">Alpha City</span></div>";

#[test]
fn footer_preserves_both_school_identities_and_unpublished_city() -> TestResult {
    let html = format!(
        "{VALID}{ROW}<div class=\"school-teaser\"><a href=\"/schools/controlled-beta\" class=\"school-teaser__title\">Controlled Beta</a></div>{ROW}<footer>MSHSL</footer>{ROW}<div class=\"pager\">Page2</div>"
    );
    let rows = parse_school_list(&html)?;
    let [first, second] = rows.as_slice() else {
        return Err("controlled listing must retain exactly its two schools".into());
    };
    check!(eq; (first.slug.as_str(), first.name.as_str(), first.city.as_deref()), ("controlled-alpha", "Controlled Alpha", Some("Alpha City")));
    check!(eq; (second.slug.as_str(), second.name.as_str(), second.city.as_deref()), ("controlled-beta", "Controlled Beta", None));
    Ok(())
}

#[test]
fn malformed_owned_row_cannot_disappear_behind_a_valid_school() -> TestResult {
    for malformed in [
        "<a href=\"/schools/missing-title\">Missing</a>",
        "<div class=\"school-teaser\"><span>Missing title and link</span></div>",
        "<span class=\"school-teaser__title\">Missing link</span>",
        "<a href=\"/schools/\" class=\"school-teaser__title\">Empty slug</a>",
        "<a href=\"/schools/a\" class=\"school-teaser__title\">A</a><a href=\"/schools/b\" class=\"school-teaser__title\">B</a>",
    ] {
        let html = format!("{VALID}{ROW}{malformed}");
        check!(matches!(parse_school_list(&html), Err(CrawlError::Schema { .. })));
    }
    Ok(())
}

#[test]
fn absent_positive_school_listing_is_not_verified_empty() -> TestResult {
    for html in [
        "",
        "<html><body>Not a directory</body></html>",
        "<div class=\"views-row\"><footer>Footer only</footer>",
    ] {
        check!(matches!(
            parse_school_list(html),
            Err(CrawlError::Schema { .. })
        ));
    }
    Ok(())
}
