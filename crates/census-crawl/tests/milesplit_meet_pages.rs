#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_crawl::milesplit::{
    parse_meet_result_files, read_meet_pages, ListedResultFile, MeetPage, MeetPageLimits,
    MeetResultFile,
};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::CrawlError;
use census_domain::UsJurisdiction;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const URL: &str = "https://oh.milesplit.com/meets/123-fixture/results";
const OTHER: &str = "https://oh.milesplit.com/meets/456-fixture/results/";
const REDIRECT: &str = "https://mi.milesplit.com/meets/789-redirect/results";
const INLINE: &str = r#"<div id="meetResultsBody"></div>"#;
const LEGACY: &str = r#"<select id="ddResultsPage"><option value="https://mi.milesplit.com/meets/789-other/results/9?heat=1">  Varsity Boys  </option></select>"#;
const SCHEMA_DETAIL: &str =
    "no meetResultFiles literal, ddResultsPage select or meetResultsBody block";

fn scratch() -> TestResult<(tempfile::TempDir, Fetcher)> {
    let dir = tempfile::tempdir()?;
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".to_string()],
    )?
    .with_offline(true);
    Ok((dir, fetcher))
}

fn seed(fetcher: &Fetcher, url: &str, body: &str) -> TestResult {
    let mut hasher = Sha256::new();
    hasher.update(b"GET\x1f");
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher
        .finalize()
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let meta = serde_json::json!({
        "url": url,
        "response_url": REDIRECT,
        "method": "GET",
        "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-20T00:00:00Z",
        "content_type": "text/html; charset=utf-8",
    });
    std::fs::create_dir_all(fetcher.cache_dir())?;
    std::fs::write(fetcher.cache_dir().join(format!("{key}.body")), body)?;
    std::fs::write(
        fetcher.cache_dir().join(format!("{key}.meta.json")),
        meta.to_string(),
    )?;
    Ok(())
}

fn options() -> FetchOptions {
    FetchOptions {
        refresh: false,
        allow_not_found: false,
        headers: Vec::new(),
    }
}

fn page(url: &str) -> MeetPage {
    MeetPage {
        results_url: url.to_string(),
        jurisdiction: UsJurisdiction::Ohio,
    }
}

fn file(id: i64, name: &str, is_meet_pro: i64, inline: bool) -> MeetResultFile {
    MeetResultFile {
        id,
        name: name.to_string(),
        is_meet_pro,
        inline,
    }
}

fn json_files(count: usize) -> String {
    let rows = vec![r#"{"id":9,"name":"Accepted"}"#; count].join(",");
    format!("let meetResultFiles = [{rows}];")
}

#[test]
fn json_preserves_names_defaults_order_duplicates_and_precedence() -> TestResult {
    let html = format!(
        r#"{INLINE}{LEGACY}<script>let meetResultFiles = [{{"id":9,"name":"  Varsity \u0042oys  ","isMeetPro":1}},{{"id":2}},{{"id":9,"name":"Girls","inline":true}}];</script>"#
    );
    check!(eq; parse_meet_result_files(URL, &html)?, vec![
        file(9, "  Varsity Boys  ", 1, false),
        file(2, "", 0, false),
        file(9, "Girls", 0, true),
    ]);
    Ok(())
}

#[test]
fn empty_json_and_empty_legacy_take_precedence_over_inline() -> TestResult {
    for html in [
        format!("{INLINE}{LEGACY}let meetResultFiles = [];"),
        format!("{INLINE}<select id=\"ddResultsPage\"></select>"),
    ] {
        check!(eq; parse_meet_result_files(URL, &html)?, Vec::<MeetResultFile>::new());
    }
    Ok(())
}

#[test]
fn legacy_preserves_positive_ids_trimmed_names_order_and_duplicates() -> TestResult {
    let html = format!(
        r#"{INLINE}<select id="ddResultsPage"><option value="https://oh.milesplit.com/meets/123-fixture/results/0">Zero</option><option value="https://oh.milesplit.com/meets/123-fixture/results/9">  Boys  </option><option value="https://oh.milesplit.com/meets/123-fixture/results/9?heat=1">Girls</option><option value="https://oh.milesplit.com/meets/123-fixture/results/10">  </option></select>"#
    );
    check!(eq; parse_meet_result_files(URL, &html)?, vec![
        file(9, "Boys", 0, false), file(9, "Girls", 0, false),
    ]);
    check!(eq; parse_meet_result_files(URL, INLINE)?, vec![file(0, "Results", 0, true)]);
    Ok(())
}

#[test]
fn malformed_json_refuses_instead_of_using_legacy_or_inline() -> TestResult {
    for literal in [
        r#"[{"id":1,"name":null}]"#,
        r#"[{"id":1,"name":3}]"#,
        r#"[{"name":"Missing id"}]"#,
        r#"[{"id":1,"inline":0}]"#,
        r#"[{"id":1},]"#,
    ] {
        let html = format!("let meetResultFiles = {literal};{LEGACY}{INLINE}");
        check!(matches!(
            parse_meet_result_files(URL, &html),
            Err(CrawlError::Decode { ref url, .. }) if url == "meet results page"
        ));
    }
    Ok(())
}

#[test]
fn json_accepts_4096_files_and_refuses_4097_with_a_large_excess_name() -> TestResult {
    let exact = json_files(4096);
    check!(eq; parse_meet_result_files(URL, &exact)?, vec![file(9, "Accepted", 0, false); 4096]);
    let rows = vec![r#"{"id":9,"name":"Accepted"}"#; 4096].join(",");
    let name = "Refused".repeat(4096);
    let excess = format!(r#"let meetResultFiles = [{rows},{{"id":10,"name":"{name}"}}];"#);
    check!(matches!(
        parse_meet_result_files(URL, &excess),
        Err(CrawlError::Resource {
            resource: "meet-page files",
            requested: 4097,
            limit: 4096,
        })
    ));
    Ok(())
}

#[test]
fn legacy_accepts_4096_files_and_refuses_4097_with_a_large_excess_name() -> TestResult {
    let rows =
        r#"<option value="https://oh.milesplit.com/meets/123-fixture/results/9">Accepted</option>"#
            .repeat(4096);
    let exact = format!(r#"<select id="ddResultsPage">{rows}</select>"#);
    check!(eq; parse_meet_result_files(URL, &exact)?, vec![file(9, "Accepted", 0, false); 4096]);
    let name = "Refused".repeat(4096);
    let excess = format!(
        r#"<select id="ddResultsPage">{rows}<option value="https://oh.milesplit.com/meets/123-fixture/results/10">{name}</option></select>"#
    );
    check!(matches!(
        parse_meet_result_files(URL, &excess),
        Err(CrawlError::Resource {
            resource: "meet-page files",
            requested: 4097,
            limit: 4096,
        })
    ));
    Ok(())
}

#[tokio::test]
async fn reader_keeps_request_owned_urls_jurisdiction_and_pro_flags() -> TestResult {
    for (html, expected) in [
        (
            r#"let meetResultFiles = [{"id":4,"isMeetPro":1},{"id":5,"inline":true}];"#,
            vec![
                ListedResultFile {
                    url: format!("{URL}/4/raw"),
                    jurisdiction: UsJurisdiction::Ohio,
                    is_meet_pro: 1,
                },
                ListedResultFile {
                    url: URL.to_string(),
                    jurisdiction: UsJurisdiction::Ohio,
                    is_meet_pro: 0,
                },
            ],
        ),
        (
            LEGACY,
            vec![ListedResultFile {
                url: format!("{URL}/9/raw"),
                jurisdiction: UsJurisdiction::Ohio,
                is_meet_pro: 0,
            }],
        ),
        (
            INLINE,
            vec![ListedResultFile {
                url: URL.to_string(),
                jurisdiction: UsJurisdiction::Ohio,
                is_meet_pro: 0,
            }],
        ),
    ] {
        let (_dir, fetcher) = scratch()?;
        seed(&fetcher, URL, html)?;
        let (result, spent) = read_meet_pages(
            &fetcher,
            [page(URL)],
            &options(),
            MeetPageLimits::standard(),
        )
        .await
        .into_parts();
        let result = result?;
        check!(eq; result.files, expected);
        check!(eq; result.pages_read, 1);
        check!(eq; result.quarantined, Vec::<(String, String)>::new());
        check!(spent > 0);
        check!(eq; fetcher.stats().await.physical_requests(), 0);
    }
    Ok(())
}

#[tokio::test]
async fn reader_refuses_inline_at_zero_remaining_files_before_name_allocation() -> TestResult {
    let (_dir, fetcher) = scratch()?;
    seed(&fetcher, URL, &json_files(4096))?;
    seed(&fetcher, OTHER, INLINE)?;
    let (exact, spent) = read_meet_pages(
        &fetcher,
        [page(URL)],
        &options(),
        MeetPageLimits::standard(),
    )
    .await
    .into_parts();
    let exact = exact?;
    check!(eq; exact.files, vec![ListedResultFile {
        url: format!("{URL}/9/raw"), jurisdiction: UsJurisdiction::Ohio, is_meet_pro: 0,
    }; 4096]);
    check!(eq; exact.pages_read, 1);
    let budget = spent.checked_add(8192).ok_or("fixture budget overflow")?;
    let (result, refused_spent) = read_meet_pages(
        &fetcher,
        [page(URL), page(OTHER)],
        &options(),
        MeetPageLimits::new(budget),
    )
    .await
    .into_parts();
    check!(matches!(
        result,
        Err(CrawlError::Resource {
            resource: "meet-page files",
            requested: 1,
            limit: 0,
        })
    ));
    check!(eq; refused_spent, budget);
    Ok(())
}

#[tokio::test]
async fn reader_refuses_allocation_and_preserves_admitted_usage() -> TestResult {
    let (_dir, fetcher) = scratch()?;
    seed(&fetcher, URL, INLINE)?;
    let (result, spent) =
        read_meet_pages(&fetcher, [page(URL)], &options(), MeetPageLimits::new(8192))
            .await
            .into_parts();
    check!(matches!(
        result,
        Err(CrawlError::Resource {
            resource: "meet-page allocation",
            requested: 8199,
            limit: 8192,
        })
    ));
    check!(eq; spent, 8192);
    Ok(())
}

#[tokio::test]
async fn reader_propagates_malformed_json_instead_of_quarantining_it() -> TestResult {
    let (_dir, fetcher) = scratch()?;
    seed(
        &fetcher,
        URL,
        r#"let meetResultFiles = [{"name":"Missing id"}];"#,
    )?;
    let (result, _) = read_meet_pages(
        &fetcher,
        [page(URL)],
        &options(),
        MeetPageLimits::standard(),
    )
    .await
    .into_parts();
    check!(matches!(result, Err(CrawlError::Decode { ref url, .. }) if url == "meet results page"));
    Ok(())
}

#[tokio::test]
async fn reader_continues_at_tied_quarantines_and_stops_only_at_a_majority() -> TestResult {
    let (_dir, fetcher) = scratch()?;
    seed(&fetcher, URL, "let meetResultFiles = [];")?;
    seed(&fetcher, OTHER, "<h1>Foreign page</h1>")?;
    let sequence = std::iter::repeat_n(page(URL), 25)
        .chain(std::iter::repeat_n(page(OTHER), 25))
        .chain([page(URL), page(OTHER), page(OTHER), page(URL)]);
    let (result, _) = read_meet_pages(&fetcher, sequence, &options(), MeetPageLimits::standard())
        .await
        .into_parts();
    let result = result?;
    check!(eq; result.pages_read, 26);
    check!(eq; result.quarantined, vec![(OTHER.to_string(), SCHEMA_DETAIL.to_string()); 27]);
    check!(eq; result.files, Vec::<ListedResultFile>::new());
    check!(result.stopped());
    Ok(())
}

#[tokio::test]
async fn reader_quarantine_threshold_allows_24_but_stops_at_25() -> TestResult {
    let (_dir, fetcher) = scratch()?;
    seed(&fetcher, URL, "let meetResultFiles = [];")?;
    seed(&fetcher, OTHER, "<h1>Foreign page</h1>")?;
    for (count, read, stopped) in [(24, 1, false), (25, 0, true)] {
        let sequence = std::iter::repeat_n(page(OTHER), count).chain([page(URL)]);
        let (result, _) =
            read_meet_pages(&fetcher, sequence, &options(), MeetPageLimits::standard())
                .await
                .into_parts();
        let result = result?;
        check!(eq; result.pages_read, read);
        check!(eq; result.quarantined, vec![(OTHER.to_string(), SCHEMA_DETAIL.to_string()); count]);
        check!(eq; result.files, Vec::<ListedResultFile>::new());
        check!(eq; result.stopped(), stopped);
    }
    Ok(())
}
