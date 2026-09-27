use super::{is_results_page, read_meet_pages, MeetPage, MISMATCH_LIMIT};
use crate::net::{FetchOptions, Fetcher};
use census_domain::UsJurisdiction;
use std::collections::HashMap;

const OH_MEET_RESULTS: &str =
    include_str!("../../../../tests/fixtures/milesplit/oh_meet_770621_results.html");
const OH_MEET_RESULTS_URL: &str =
    "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results";
const FOREIGN_PAGE: &str = "<html><body><h1>Schedule</h1><p>Not a results page.</p></body></html>";

fn scratch() -> (tempfile::TempDir, Fetcher) {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher");
    (dir, fetcher)
}

fn options() -> FetchOptions {
    FetchOptions {
        refresh: false,
        allow_not_found: false,
        headers: Vec::new(),
    }
}

fn seed_cache(cache_dir: &std::path::Path, url: &str, status: u16, body: &str) {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher.finalize()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let meta = serde_json::json!({
        "url": url,
        "method": "GET",
        "status": status,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-24T03:55:59Z",
    });
    std::fs::create_dir_all(cache_dir).expect("cache dir");
    std::fs::write(
        cache_dir.join(format!("{key}.meta.json")),
        serde_json::to_string(&meta).expect("meta json"),
    )
    .expect("write meta");
    std::fs::write(cache_dir.join(format!("{key}.body")), body).expect("write body");
}

fn page(url: &str) -> MeetPage {
    MeetPage {
        results_url: url.to_string(),
        jurisdiction: UsJurisdiction::Ohio,
    }
}

#[tokio::test]
async fn an_unreadable_page_is_quarantined_and_the_walk_goes_on() {
    let (dir, fetcher) = scratch();
    let cache = dir.path().join("http");
    seed_cache(&cache, OH_MEET_RESULTS_URL, 200, OH_MEET_RESULTS);
    let foreign = "https://oh.milesplit.com/meets/999999-schedule/results";
    seed_cache(&cache, foreign, 200, FOREIGN_PAGE);

    let pages = read_meet_pages(
        &fetcher,
        vec![page(foreign), page(OH_MEET_RESULTS_URL)],
        &options(),
    )
    .await
    .expect("a template mismatch is not an error the walk returns");

    assert_eq!(pages.quarantined.len(), 1, "{:?}", pages.quarantined);
    assert_eq!(pages.quarantined[0].0, foreign);
    assert!(
        !pages.quarantined[0].1.is_empty(),
        "the parser's own reason travels with the URL"
    );
    assert_eq!(pages.pages_read, 1);
    assert!(
        !pages.files.is_empty(),
        "the readable page's files are still collected"
    );
    assert!(
        !pages.stopped(),
        "one junk page among readable ones is not a stop"
    );
}

#[tokio::test]
async fn an_unreachable_page_is_not_read_as_a_meet_without_results() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (_dir, fetcher) = scratch();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("loopback listener");
    let failing = format!("http://{}/meets/888888-invite-2026/results", listener.local_addr().expect("address"));
    let options = options();
    let serve = async {
        for (status, body) in [("200 OK", "User-agent: *\nAllow: /\n"), ("500 Internal Server Error", "server error")] {
            let (mut stream, _) = listener.accept().await.expect("request connection");
            let mut request = [0_u8; 8192];
            let mut filled = 0;
            for _ in 0..request.len() {
                let count = stream.read(&mut request[filled..]).await.expect("request headers");
                filled += count;
                if count == 0 || request[..filled].ends_with(b"\r\n\r\n") { break; }
            }
            assert!(request[..filled].ends_with(b"\r\n\r\n"), "bounded HTTP request headers");
            let response = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            stream.write_all(response.as_bytes()).await.expect("fixture response");
        }
    };
    let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(read_meet_pages(&fetcher, vec![page(&failing)], &options), serve)
    }).await.expect("bounded local fixture");
    let error = result.expect_err("a fetch failure propagates");
    assert!(matches!(error, crate::CrawlError::Fetch(crate::net::FetchError::Http { status: 500, ref url }) if url == &failing),
        "expected the origin's HTTP failure, got {error:?}");
}

#[tokio::test]
async fn mismatches_without_readable_pages_stop_the_walk() {
    let (dir, fetcher) = scratch();
    let cache = dir.path().join("http");
    let junk: Vec<String> = (0..MISMATCH_LIMIT)
        .map(|n| {
            format!(
                "https://oh.milesplit.com/meets/{}-junk/results",
                700_000 + n
            )
        })
        .collect();
    for url in &junk {
        seed_cache(&cache, url, 200, FOREIGN_PAGE);
    }
    seed_cache(&cache, OH_MEET_RESULTS_URL, 200, OH_MEET_RESULTS);

    let mut walked = junk.clone();
    walked.push(OH_MEET_RESULTS_URL.to_string());
    let pages = read_meet_pages(&fetcher, walked.iter().map(|url| page(url)), &options())
        .await
        .expect("mismatches are quarantined, not returned");

    assert!(pages.stopped(), "a majority-mismatch run stops");
    assert_eq!(pages.pages_read, 0);
    assert_eq!(pages.quarantined.len(), MISMATCH_LIMIT);
    assert!(
        pages.files.is_empty(),
        "the walk stopped before reaching the readable page"
    );
}

#[test]
fn only_a_milesplit_results_page_belongs_to_this_reader() {
    assert!(is_results_page(OH_MEET_RESULTS_URL));
    assert!(is_results_page("https://www.milesplit.com/meets/1/results"));
    assert!(is_results_page("https://oh.milesplit.com/meets/1/results/"));
    assert!(!is_results_page(
        "https://www.athletic.net/TrackAndField/meet/634313/results"
    ));
    assert!(!is_results_page("https://www.wiaa.com/tournament/results"));
    assert!(!is_results_page("https://oh.milesplit.com/meets/1"));
    assert!(!is_results_page("not a url"));
}
