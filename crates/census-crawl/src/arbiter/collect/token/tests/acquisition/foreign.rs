use super::{acquire_bundle, archived, fetcher, reply, serve, FetchOptions, TestResult};
use std::time::Duration;

#[test]
fn unmatched_foreign_markup_requests_only_the_genuine_entry_bundle() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let root = tempfile::tempdir()?;
    let cache = root.path().join("http");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let requested = format!("{origin}/directory/");
    let selected = format!("{origin}/directory/assets/index-real.js");
    let html = "<svg></math><script type=module src='assets/index-decoy.js'></script></svg><math/><script type=module src='assets/index-real.js'></script>";
    let body = "export const sourceRevision = 'genuine';";
    let replies = vec![
        reply("/robots.txt", 200, "User-agent: *\r\nAllow: /\r\n"),
        reply("/directory/", 200, html),
        reply("/directory/assets/index-real.js", 200, body),
    ];
    let client = async {
        let client = fetcher(&cache)?;
        let bundle = acquire_bundle(&client, &requested, &FetchOptions::default()).await?;
        check!(eq; bundle.url, selected);
        check!(eq; bundle.response_url.as_deref(), Some(selected.as_str()));
        check!(eq; bundle.body, body.as_bytes());
        archived(&cache, &requested, Some(&requested), html.as_bytes())?;
        archived(&cache, &selected, Some(&selected), body.as_bytes())?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        let (served, acquired) = tokio::join!(serve(listener, replies), client);
        served?;
        acquired
    }).await?
    })
}
