#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_crawl::athleticnet::{parse_mark, Bio, TfRow};
use census_crawl::coach_directories::{parse_directory, parse_summary};
use census_crawl::hytek::{parse_field_mark, parse_time};
use census_crawl::{bound, chsaa, ciac, home_campus, CrawlError};
use census_domain::model::EventKind;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const DIRECTORY_ROW_LIMIT: usize = 20_000;

#[test]
fn an_unterminated_title_tag_names_no_school_and_carries_no_coaches() -> TestResult {
    let parsed = bound::parse("<title>Central High | Track")?;
    check!(
        parsed.page.title_school.is_empty(),
        "an unterminated <title> must not name a school: {:?}",
        parsed.page.title_school
    );
    check!(
        parsed.coaches.is_empty(),
        "an unterminated <title> must not produce coaches: {:?}",
        parsed.coaches
    );
    Ok(())
}

#[test]
fn attribute_soup_around_a_coaching_row_retains_exactly_the_published_coach() -> TestResult {
    let html = concat!(
        "<title>x|Central High|Cross Country</title>",
        "Coaching Staff",
        "<tr class=\"odd\" data-row=\"1\">",
        "<td width=\"40%\" data-role=\"coach\">Head Coach</td>",
        "<td><span class=\"name\">Jo Smith</span></td>",
        "</tr></table>"
    );
    let parsed = bound::parse(html)?;
    check!(
        eq;
        parsed.coaches.len(),
        1,
        "attribute soup must not multiply or drop the published row"
    );
    check!(eq; parsed.coaches[0].role, "Head Coach");
    check!(eq; parsed.coaches[0].name, "Jo Smith");
    Ok(())
}

#[test]
fn oversized_nested_tag_soup_names_no_school_and_carries_no_coaches() -> TestResult {
    let mut html = String::with_capacity(900_000);
    for _ in 0..20_000 {
        html.push_str("<div class='DirectoryDetail'><span data-x='1'><b>");
    }
    let parsed = bound::parse(&html)?;
    check!(
        parsed.page.title_school.is_empty(),
        "nested tag soup must not name a school"
    );
    check!(
        parsed.coaches.is_empty(),
        "nested tag soup must not produce coaches: {}",
        parsed.coaches.len()
    );
    Ok(())
}

#[test]
fn a_truncated_staff_row_is_dropped_while_the_complete_row_survives() -> TestResult {
    let html = concat!(
        "<tr><td>Boys Cross Country</td><td>Coach</td><td>Jo Smith</td></tr>",
        "<tr><td>Boys Cross Country</td><td>Coach</td><td>Ka"
    );
    let table = match ciac::pages::parse_table(html)? {
        Some(table) => table,
        None => return Err("a complete published row must still parse".into()),
    };
    check!(
        eq;
        table.rows.len(),
        1,
        "the truncated row must not be admitted: {:?}",
        table.rows
    );
    check!(eq; table.rows[0].0, "Boys Cross Country");
    check!(eq; table.rows[0].1, "Jo Smith");
    Ok(())
}

#[test]
fn a_header_only_or_entirely_truncated_table_carries_no_rows() -> TestResult {
    check!(
        ciac::pages::parse_table("<tr><th>Sport</th><th>Coach</th></tr>")?.is_none(),
        "a header-only table must not carry a coach row"
    );
    check!(
        ciac::pages::parse_table("<tr><td>Boys Cross Country</td><td>Coach</td><td>")?.is_none(),
        "a table truncated before any complete row must carry no rows"
    );
    Ok(())
}

#[test]
fn directory_tables_are_read_at_their_published_shape_only() -> TestResult {
    let published = concat!(
        "<div class='DirectoryDetail'><b>Central High</b></div>",
        "<table class='DirectoryStaffTable'>",
        "<tr><td>Boys Cross Country</td><td>Coach</td><td>Jo Smith</td></tr>",
        "</table>"
    );
    let results = ciac::parse_directory(published)?;
    check!(
        eq;
        results.len(),
        1,
        "the published directory shape must yield exactly one table: {:?}",
        results
    );
    check!(eq; results[0].0, "Central High");
    check!(eq; results[0].1.rows.len(), 1);

    let attributed = concat!(
        "<div class='DirectoryDetail'><b>Central High</b></div>",
        "<table class='DirectoryStaffTable' data-x='1'>",
        "<tr><td>Boys Cross Country</td><td>Coach</td><td>Jo Smith</td></tr>",
        "</table>"
    );
    check!(
        ciac::parse_directory(attributed)?.is_empty(),
        "only the exact published class tag is recognized, not an attributed variant"
    );
    Ok(())
}

#[test]
fn an_unattributed_staff_table_admits_no_rows_at_any_size() -> TestResult {
    let mut html = String::with_capacity(1_100_000);
    html.push_str("<table class='DirectoryStaffTable'>");
    for _ in 0..30_000 {
        html.push_str("<tr><td></td><td></td><td></td></tr>");
    }
    html.push_str("</table>");
    check!(
        ciac::parse_directory(&html)?.is_empty(),
        "a table without a published school name must not be admitted"
    );
    check!(
        ciac::pages::parse_table(&html)?.is_none(),
        "cells carrying no published text must not become rows"
    );
    Ok(())
}

#[test]
fn a_published_button_link_is_read_once_and_deduplicated_by_id() -> TestResult {
    let links = home_campus::parse_directory_links(
        "<button data-id=\"7\">Alpha</button><button data-id=\"7\">Beta</button>",
    );
    check!(eq; links.len(), 1, "a repeated published id must stay one link");
    check!(eq; links[0].id, 7u64);
    check!(eq; links[0].name, "Alpha");
    Ok(())
}

#[test]
fn a_hostile_button_id_or_unterminated_value_mints_no_school_link() -> TestResult {
    for html in [
        "<button data-id=\"not-a-number\">Alpha</button>",
        "<button data-id=\"7>Alpha</button>",
        "<button data-id=\"7\"></button>",
        "<button data-id=\"\">Alpha</button>",
    ] {
        let links = home_campus::parse_directory_links(html);
        check!(
            links.is_empty(),
            "hostile attribute soup must mint no school link: {html:?} => {links:?}"
        );
    }
    Ok(())
}

#[test]
fn a_details_row_with_a_published_id_survives_a_thin_shape() -> TestResult {
    let details = match home_campus::parse_school_details("{\"school\":{\"id\":19}}") {
        Some(details) => details,
        None => return Err("a published school id must not be refused".into()),
    };
    check!(eq; details.profile.id, 19u64);
    check!(details.profile.name.is_empty());
    check!(details.coaches.is_empty());
    check!(details.faculties.is_empty());
    Ok(())
}

#[test]
fn a_contradictory_details_row_refuses_the_whole_record() -> TestResult {
    for json in [
        "{\"school\":{\"id\":\"19\"}}",
        "{\"school\":{\"id\":19",
        "{\"school\":{\"id\":19.5}}",
        "",
    ] {
        check!(
            home_campus::parse_school_details(json).is_none(),
            "a contradictory details row must refuse: {json:?}"
        );
    }
    Ok(())
}

#[test]
fn an_unbalanced_chsaa_directory_array_refuses_instead_of_panicking() -> TestResult {
    for body in [
        r#"[{\"schoolCode\":1"#,
        r#"<html><body>no member array</body></html>"#,
        "",
    ] {
        match chsaa::parse::parse_directory(body) {
            Err(CrawlError::Invariant { .. }) => {}
            Err(other) => {
                return Err(format!(
                    "an unbalanced directory array must refuse as an invariant error: {body:?} => {other:?}"
                )
                .into())
            }
            Ok(_) => {
                return Err(format!(
                    "an unbalanced directory array must refuse instead of being accepted: {body:?}"
                )
                .into())
            }
        }
    }
    Ok(())
}

#[test]
fn a_directory_page_that_is_not_utf8_refuses_as_a_decode_error() -> TestResult {
    let outcome = parse_directory(&[0xff, 0xfe, 0x00, 0x7b]);
    check!(
        matches!(outcome, Err(CrawlError::Decode { .. })),
        "invalid UTF-8 must be a typed decode refusal: {outcome:?}"
    );
    let summary = parse_summary(&[0xc3, 0x28]);
    check!(
        matches!(summary, Err(CrawlError::Decode { .. })),
        "invalid UTF-8 must be a typed decode refusal: {summary:?}"
    );
    Ok(())
}

#[test]
fn a_directory_page_from_an_unknown_shape_refuses_as_a_decode_error() -> TestResult {
    for body in [
        r#"{"currentPage":1,"totalPages":1,"totalResults":0}"#,
        r#"{"current_page":1,"total_pages":1,"total_results":0,"results":[]}"#,
        r#"{"currentPage":1,"totalPages":1,"totalResults":0,"results":{}}"#,
    ] {
        let outcome = parse_directory(body.as_bytes());
        check!(
            matches!(outcome, Err(CrawlError::Decode { .. })),
            "an older or unknown page shape must refuse: {body} => {outcome:?}"
        );
    }
    Ok(())
}

#[test]
fn a_directory_page_with_zero_identity_refuses_as_a_schema_error() -> TestResult {
    let outcome =
        parse_directory(br#"{"currentPage":0,"totalPages":1,"totalResults":0,"results":[]}"#);
    check!(
        matches!(outcome, Err(CrawlError::Schema { .. })),
        "a zero page identity must refuse: {outcome:?}"
    );
    Ok(())
}

#[test]
fn a_directory_row_that_is_not_a_published_object_refuses_the_page() -> TestResult {
    for body in [
        r#"{"currentPage":1,"totalPages":1,"totalResults":1,"results":[1]}"#,
        r#"{"currentPage":1,"totalPages":1,"totalResults":1,"results":[null]}"#,
        r#"{"currentPage":1,"totalPages":1,"totalResults":1,"results":["Central"]}"#,
    ] {
        let outcome = parse_directory(body.as_bytes());
        check!(
            matches!(outcome, Err(CrawlError::Decode { .. })),
            "a row that is not a published object must refuse the page: {body} => {outcome:?}"
        );
    }
    Ok(())
}

#[test]
fn an_additively_extended_directory_row_keeps_its_published_fields() -> TestResult {
    let body = br#"{"currentPage":1,"totalPages":1,"totalResults":1,"results":[{"name":"Central High","district":"AL-042","city":"Springville"}]}"#;
    let page = parse_directory(body)?;
    check!(eq; page.results.len(), 1);
    check!(eq; page.results[0].name.as_deref(), Some("Central High"));
    check!(eq; page.results[0].city.as_deref(), Some("Springville"));
    check!(
        page.results[0].postal_issues.is_empty(),
        "an unknown additive key must not be read as a contradiction: {:?}",
        page.results[0].postal_issues
    );
    Ok(())
}

#[test]
fn directory_row_count_is_accepted_at_the_limit_and_refused_over_it() -> TestResult {
    let rows_at_limit = vec!["{}"; DIRECTORY_ROW_LIMIT].join(",");
    let at_limit = format!(
        r#"{{"currentPage":1,"totalPages":1,"totalResults":{DIRECTORY_ROW_LIMIT},"results":[{rows_at_limit}]}}"#
    );
    let outcome = parse_directory(at_limit.as_bytes());
    check!(
        outcome.is_ok(),
        "exactly {DIRECTORY_ROW_LIMIT} published rows must be accepted: {outcome:?}"
    );
    let rows_over = vec!["{}"; DIRECTORY_ROW_LIMIT + 1].join(",");
    let over = format!(
        r#"{{"currentPage":1,"totalPages":1,"totalResults":{},"results":[{rows_over}]}}"#,
        DIRECTORY_ROW_LIMIT + 1
    );
    let outcome = parse_directory(over.as_bytes());
    check!(
        matches!(outcome, Err(CrawlError::Schema { .. })),
        "more than {DIRECTORY_ROW_LIMIT} published rows must refuse: {outcome:?}"
    );
    Ok(())
}

#[test]
fn contradictory_published_postal_fields_are_retained_as_explicit_issues() -> TestResult {
    let body =
        br#"{"currentPage":1,"totalPages":1,"totalResults":1,"results":[{"city":"Ka\u0000nsas","address":42}]}"#;
    let page = parse_directory(body)?;
    check!(eq; page.results.len(), 1);
    let row = &page.results[0];
    check!(
        row.city.is_none(),
        "a published city carrying a control character must not be admitted"
    );
    check!(
        row.address.is_none(),
        "a published address that is not text must not be admitted"
    );
    check!(
        eq;
        row.postal_issues.len(),
        2,
        "both contradictions must stay explicit: {:?}",
        row.postal_issues
    );
    check!(
        row.postal_issues
            .iter()
            .any(|issue| issue.contains("contains control characters")),
        "the refused city must be recorded: {:?}",
        row.postal_issues
    );
    check!(
        row.postal_issues
            .iter()
            .any(|issue| issue.contains("must be published text or null")),
        "the refused address must be recorded: {:?}",
        row.postal_issues
    );
    Ok(())
}

#[test]
fn a_summary_address_from_an_unknown_shape_is_retained_as_an_explicit_issue() -> TestResult {
    let summary = parse_summary(br#"{"id":"S1","address":"21 Hayestown Avenue"}"#)?;
    check!(summary.address.address1.is_none());
    check!(summary.address.city.is_none());
    check!(
        eq;
        summary.address.postal_issues.len(),
        1,
        "one refusal must be recorded: {:?}",
        summary.address.postal_issues
    );
    check!(
        summary.address.postal_issues[0].contains("must be a published object"),
        "the refused summary address must say why: {:?}",
        summary.address.postal_issues
    );

    let absent = parse_summary(br#"{"id":"S1","address":null}"#)?;
    check!(
        absent.address.postal_issues.is_empty(),
        "a null published address is absence, not a contradiction: {:?}",
        absent.address.postal_issues
    );
    Ok(())
}

#[test]
fn an_athlete_bio_row_without_a_published_identifier_refuses() -> TestResult {
    check!(serde_json::from_str::<Bio>(r#"{"athlete":{}}"#).is_err());
    check!(serde_json::from_str::<Bio>(r#"{"athlete":{"IDAthlete":"28872883"}}"#).is_err());
    check!(serde_json::from_slice::<Bio>(&[0xff, 0xfe]).is_err());
    check!(serde_json::from_str::<Bio>(r#"{"athlete":{"IDAthlete":28872883}}"#).is_ok());
    Ok(())
}

#[test]
fn a_contradictory_published_place_refuses_the_row() -> TestResult {
    check!(serde_json::from_str::<TfRow>(r#"{"IDResult":7,"Place":{"nested":true}}"#).is_err());
    check!(serde_json::from_str::<TfRow>(r#"{"IDResult":7,"Place":[1]}"#).is_err());
    check!(serde_json::from_str::<TfRow>(r#"{"IDResult":7,"Place":true}"#).is_err());
    Ok(())
}

#[test]
fn published_place_forms_are_read_without_invention() -> TestResult {
    let text = serde_json::from_str::<TfRow>(r#"{"IDResult":7,"Place":"3"}"#)?;
    check!(eq; text.place.as_deref(), Some("3"));
    let number = serde_json::from_str::<TfRow>(r#"{"IDResult":7,"Place":12}"#)?;
    check!(eq; number.place.as_deref(), Some("12"));
    let blank = serde_json::from_str::<TfRow>(r#"{"IDResult":7,"Place":"  "}"#)?;
    check!(
        blank.place.is_none(),
        "a blank published place must be absent"
    );
    let missing = serde_json::from_str::<TfRow>(r#"{"IDResult":7}"#)?;
    check!(missing.place.is_none());
    Ok(())
}

#[test]
fn no_mark_zero_and_non_numeric_published_tokens_refuse_a_mark() -> TestResult {
    for published in ["", "DQ", "0", "0.0", "abc", "  ", ".5"] {
        let outcome = parse_mark(&EventKind::Track1600m, published);
        check!(
            outcome.is_none(),
            "published {published:?} must not manufacture a mark: {outcome:?}"
        );
    }
    for published in ["", "DQ", "0", "abc"] {
        let outcome = parse_mark(&EventKind::HighJump, published);
        check!(
            outcome.is_none(),
            "published {published:?} must not manufacture a field mark: {outcome:?}"
        );
    }
    Ok(())
}

#[test]
fn published_times_and_imperial_fields_stay_readable_at_their_real_forms() -> TestResult {
    check!(parse_mark(&EventKind::Track1600m, "4:12.30").is_some());
    check!(parse_mark(&EventKind::Track1600m, "4:12.30a").is_some());
    check!(parse_mark(&EventKind::HighJump, "1.85").is_some());
    check!(parse_mark(&EventKind::HighJump, "1' 11\"").is_some());
    check!(parse_mark(&EventKind::HighJump, "1-11").is_some());
    check!(parse_mark(&EventKind::Decathlon, "6240").is_some());
    Ok(())
}

#[test]
fn contradictory_published_marks_refuse() -> TestResult {
    check!(
        parse_mark(&EventKind::HighJump, "1-13").is_none(),
        "an imperial mark with 13 inches must refuse"
    );
    check!(
        parse_time("12:99").is_none(),
        "a mark with 99 seconds must refuse"
    );
    check!(parse_time("1:60").is_none(), "sixty seconds is invalid");
    check!(parse_time("").is_none());
    check!(parse_field_mark("").is_none());
    check!(
        parse_field_mark("1-13").is_none(),
        "thirteen inches must refuse"
    );
    Ok(())
}
