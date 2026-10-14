use super::super::tests::{options, scratch, seed_cache};
use super::*;
use crate::milesplit::results::pages::ListedResultFile;
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const URL: &str = "https://oh.milesplit.com/meets/123-fixture/results";

fn page() -> MeetPage {
    MeetPage {
        results_url: URL.to_string(),
        jurisdiction: UsJurisdiction::Ohio,
    }
}

#[tokio::test]
async fn bounded_discovery_preserves_literal_legacy_and_inline_url_rules() -> TestResult {
    for html in [
        "let meetResultFiles = [{\"id\":4,\"name\":\"Results\",\"isMeetPro\":0}];",
        "<select id=\"ddResultsPage\"><option value=\"https://oh.milesplit.com/meets/123-fixture/results/4\">Results</option></select>",
        "<div id=\"meetResultsBody\">Results</div>",
    ] {
        let (_dir, fetcher) = scratch()?;
        seed_cache(fetcher.cache_dir(), URL, 200, html)?;
        let envelope = read_meet_pages(&fetcher, [page()], &options(), MeetPageLimits::new(64 * 1024 * 1024)).await;
        let (bounded, requested) = envelope.into_parts();
        let bounded = bounded?;
        let url = if html.contains("meetResultsBody") {
            URL.to_string()
        } else {
            format!("{URL}/4/raw")
        };
        check!(eq; bounded.files, vec![ListedResultFile {
            url, jurisdiction: UsJurisdiction::Ohio, is_meet_pro: 0,
        }]);
        check!(eq; bounded.pages_read, 1);
        check!(requested > 0);
    }
    Ok(())
}

#[tokio::test]
async fn bounded_discovery_file_cap_and_allocation_refusal_preserve_spent_usage() -> TestResult {
    let (_dir, fetcher) = scratch()?;
    let files = (0..4097)
        .map(|id| format!("{{\"id\":{id},\"name\":\"Results\"}}"))
        .collect::<Vec<_>>()
        .join(",");
    seed_cache(
        fetcher.cache_dir(),
        URL,
        200,
        &format!("let meetResultFiles = [{files}];"),
    )?;
    let (outcome, requested) = read_meet_pages(
        &fetcher,
        [page()],
        &options(),
        MeetPageLimits::new(64 * 1024 * 1024),
    )
    .await
    .into_parts();
    check!(matches!(
        outcome,
        Err(CrawlError::Resource {
            resource: "meet-page files",
            requested: 4097,
            limit: 4096
        })
    ));
    check!(requested > 0);
    let (outcome, spent) =
        read_meet_pages(&fetcher, [page()], &options(), MeetPageLimits::new(8192))
            .await
            .into_parts();
    check!(matches!(
        outcome,
        Err(CrawlError::Resource {
            resource: "meet-page allocation",
            ..
        })
    ));
    check!(eq; spent, 8192);
    Ok(())
}

#[tokio::test]
async fn bounded_discovery_schema_quarantine_retains_usage_and_stops_at_existing_limit(
) -> TestResult {
    let (_dir, fetcher) = scratch()?;
    seed_cache(fetcher.cache_dir(), URL, 200, "<h1>foreign page</h1>")?;
    let (outcome, requested) = read_meet_pages(
        &fetcher,
        (0..26).map(|_| page()),
        &options(),
        MeetPageLimits::new(64 * 1024 * 1024),
    )
    .await
    .into_parts();
    let outcome = outcome?;
    check!(outcome.stopped());
    check!(eq; outcome.quarantined.len(), super::super::MISMATCH_LIMIT);
    check!(eq; outcome.pages_read, 0);
    check!(requested > 0);
    Ok(())
}

#[test]
fn bounded_raw_urls_accept_exact_cap_before_growth_and_match_existing_encoder() -> TestResult {
    let file = crate::milesplit::MeetResultFile {
        id: i64::MIN,
        name: String::new(),
        is_meet_pro: 0,
        inline: false,
    };
    let base = "a".repeat(4096 - 25);
    let mut allocation = Allocation {
        limit: 8192,
        requested: 0,
    };
    let actual = raw_url(&file, &base, &mut allocation)?;
    check!(eq; actual.len(), 4096);
    check!(eq; actual, file.raw_url(&base));
    let before = allocation.requested;
    check!(raw_url(&file, &format!("{base}a"), &mut allocation).is_err());
    check!(eq; allocation.requested, before);
    Ok(())
}

#[test]
fn admitted_single_buffer_lossy_decode_matches_original_byte_replacement_semantics() -> TestResult {
    for bytes in [
        b"\xffa\xfe".as_slice(),
        b"a\xe2\x82".as_slice(),
        b"\xf0\x9f\x92\xa9\xff".as_slice(),
        b"\xc0\xaf\xed\xa0\x80".as_slice(),
    ] {
        let capacity = bytes.len().checked_mul(3).ok_or("capacity")?;
        let mut allocation = Allocation {
            limit: u64::try_from(capacity)?,
            requested: 0,
        };
        let actual = lossy_body(bytes, &mut allocation)?;
        check!(eq; actual, String::from_utf8_lossy(bytes));
        check!(eq; allocation.requested, u64::try_from(capacity)?);
        let mut below = Allocation {
            limit: u64::try_from(capacity.checked_sub(1).ok_or("capacity")?)?,
            requested: 0,
        };
        check!(lossy_body(bytes, &mut below).is_err());
        check!(eq; below.requested, 0);
    }
    Ok(())
}

#[test]
fn canonical_file_parser_preserves_names_defaults_order_and_duplicates() -> TestResult {
    let html = r#"let meetResultFiles = [{"id":9,"name":"  Varsity \u0042oys  ","isMeetPro":1},{"id":2},{"id":9,"name":"Girls","inline":true}];"#;
    let files = crate::milesplit::parse_meet_result_files(URL, html)?;
    check!(eq; files, vec![
        crate::milesplit::MeetResultFile { id: 9, name: "  Varsity Boys  ".to_string(), is_meet_pro: 1, inline: false },
        crate::milesplit::MeetResultFile { id: 2, name: String::new(), is_meet_pro: 0, inline: false },
        crate::milesplit::MeetResultFile { id: 9, name: "Girls".to_string(), is_meet_pro: 0, inline: true },
    ]);
    Ok(())
}

#[test]
fn canonical_file_parser_preserves_legacy_positive_ids_and_inline_name() -> TestResult {
    let html = r#"<select id="ddResultsPage"><option value="https://oh.milesplit.com/meets/123-fixture/results/0">Zero</option><option value="https://oh.milesplit.com/meets/123-fixture/results/9">  Varsity Boys  </option><option value="https://oh.milesplit.com/meets/123-fixture/results/9?heat=1">Girls</option></select>"#;
    let files = crate::milesplit::parse_meet_result_files(URL, html)?;
    check!(eq; files, vec![
        crate::milesplit::MeetResultFile { id: 9, name: "Varsity Boys".to_string(), is_meet_pro: 0, inline: false },
        crate::milesplit::MeetResultFile { id: 9, name: "Girls".to_string(), is_meet_pro: 0, inline: false },
    ]);
    let files =
        crate::milesplit::parse_meet_result_files(URL, r#"<div id="meetResultsBody"></div>"#)?;
    check!(eq; files, vec![crate::milesplit::MeetResultFile { id: 0, name: "Results".to_string(), is_meet_pro: 0, inline: true }]);
    Ok(())
}

#[test]
fn canonical_file_parser_rejects_malformed_json_without_html_fallback() -> TestResult {
    for literal in [
        r#"[{"id":1,"name":null}]"#,
        r#"[{"id":1,"name":3}]"#,
        r#"[{"name":"Missing id"}]"#,
        r#"[{"id":1,"inline":0}]"#,
    ] {
        let html = format!("let meetResultFiles = {literal}; <div id=\"meetResultsBody\"></div>");
        check!(
            matches!(crate::milesplit::parse_meet_result_files(URL, &html), Err(CrawlError::Decode { ref url, .. }) if url == "meet results page")
        );
    }
    check!(eq; crate::milesplit::parse_meet_result_files(URL, "let meetResultFiles = [];")?, Vec::new());
    Ok(())
}

#[test]
fn canonical_file_names_charge_allocation_before_storage() -> TestResult {
    let html = r#"let meetResultFiles = [{"id":4,"name":"Varsity"}];"#;
    let mut full = Allocation::new(MeetPageLimits::standard());
    let expected = crate::milesplit::parse::meet_index::parse_bounded(URL, html, 4096, &mut full)?;
    let mut exact = Allocation::new(MeetPageLimits::new(full.requested));
    check!(eq; crate::milesplit::parse::meet_index::parse_bounded(URL, html, 4096, &mut exact)?, expected);
    let mut below = Allocation::new(MeetPageLimits::new(
        full.requested.checked_sub(1).ok_or("budget")?,
    ));
    check!(matches!(
        crate::milesplit::parse::meet_index::parse_bounded(URL, html, 4096, &mut below),
        Err(CrawlError::Resource {
            resource: "meet-page allocation",
            ..
        })
    ));
    check!(below.requested < full.requested);
    Ok(())
}

#[tokio::test]
async fn canonical_reader_keeps_majority_quarantine_rule_after_twenty_five_successes() -> TestResult
{
    let (_dir, fetcher) = scratch()?;
    let good = format!("{URL}-good");
    seed_cache(fetcher.cache_dir(), &good, 200, "let meetResultFiles = [];")?;
    seed_cache(fetcher.cache_dir(), URL, 200, "<h1>Foreign</h1>")?;
    let pages = (0..26)
        .map(|_| MeetPage {
            results_url: good.clone(),
            jurisdiction: UsJurisdiction::Ohio,
        })
        .chain((0..28).map(|_| page()));
    let (outcome, _) = read_meet_pages(&fetcher, pages, &options(), MeetPageLimits::standard())
        .await
        .into_parts();
    let outcome = outcome?;
    check!(eq; outcome.pages_read, 26);
    check!(eq; outcome.quarantined.len(), 27);
    check!(outcome.stopped());
    check!(outcome.files.is_empty());
    Ok(())
}

#[tokio::test]
async fn canonical_reader_does_not_quarantine_valid_allocation_refusal() -> TestResult {
    let (_dir, fetcher) = scratch()?;
    seed_cache(
        fetcher.cache_dir(),
        URL,
        200,
        r#"let meetResultFiles = [{"id":1,"name":"Results"}];"#,
    )?;
    let (outcome, spent) =
        read_meet_pages(&fetcher, [page()], &options(), MeetPageLimits::new(8192))
            .await
            .into_parts();
    check!(matches!(
        outcome,
        Err(CrawlError::Resource {
            resource: "meet-page allocation",
            ..
        })
    ));
    check!(eq; spent, 8192);
    Ok(())
}

#[test]
fn legacy_file_cap_refuses_excess_name_before_its_allocation() -> TestResult {
    let accepted =
        "<option value=\"https://oh.milesplit.com/meets/123-fixture/results/9\">Accepted</option>"
            .repeat(MAX_FILES);
    let exact_html = format!("<select id=\"ddResultsPage\">{accepted}</select>");
    let mut exact = Allocation::new(MeetPageLimits::standard());
    let files = crate::milesplit::parse::meet_index::parse_bounded(
        URL,
        &exact_html,
        MAX_FILES,
        &mut exact,
    )?;
    check!(eq; files.len(), MAX_FILES);
    check!(files.iter().all(|file| file.id == 9
        && file.name == "Accepted"
        && file.is_meet_pro == 0
        && !file.inline));
    let refused_name = "Refused".repeat(4096);
    let excess_html = format!("<select id=\"ddResultsPage\">{accepted}<option value=\"https://oh.milesplit.com/meets/123-fixture/results/10\">{refused_name}</option></select>");
    let mut excess = Allocation::new(MeetPageLimits::new(exact.requested));
    let outcome = crate::milesplit::parse::meet_index::parse_bounded(
        URL,
        &excess_html,
        MAX_FILES,
        &mut excess,
    );
    check!(matches!(
        outcome,
        Err(CrawlError::Resource {
            resource: "meet-page files",
            requested: 4097,
            limit: 4096,
        })
    ));
    check!(eq; excess.requested, exact.requested);
    Ok(())
}

#[test]
fn inline_file_zero_allowance_refuses_before_name_allocation() -> TestResult {
    let mut allocation = Allocation::new(MeetPageLimits::new(8192));
    let outcome = crate::milesplit::parse::meet_index::parse_bounded(
        URL,
        r#"<div id="meetResultsBody"></div>"#,
        0,
        &mut allocation,
    );
    check!(matches!(
        outcome,
        Err(CrawlError::Resource {
            resource: "meet-page files",
            requested: 1,
            limit: 0,
        })
    ));
    check!(eq; allocation.requested, 8192);
    Ok(())
}
