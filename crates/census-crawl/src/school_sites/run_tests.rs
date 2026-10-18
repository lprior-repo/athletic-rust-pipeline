use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::Fetcher;
use crate::school_sites::{crawl_site, QueueRow, Rules};
use std::collections::HashMap;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn cache(fetcher: &Fetcher, requested: &str, actual: &str, body: &[u8], status: u16) -> TestResult {
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", requested, ""));
    let meta = CacheMeta {
        url: requested.to_owned(),
        response_url: Some(actual.to_owned()),
        method: "GET".to_owned(),
        status,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: crate::net::now_iso8601(),
        ..CacheMeta::default()
    };
    write_cache(&body_path, &meta_path, body, &meta)?;
    Ok(())
}

#[tokio::test]
async fn redirected_hits_bind_exact_captured_url_without_cross_capture_aliasing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = Fetcher::new(
        dir.path(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["school.example".to_owned()],
    )?
    .with_offline(true);
    cache(
        &fetcher,
        "https://school.example/robots.txt",
        "https://school.example/robots.txt",
        b"",
        200,
    )?;
    let first = b"<table><tr><th>Sport</th><th>Coach</th><th>E-mail</th></tr><tr><td>Cross Country - Girls</td><td>Mary Ann Lee</td><td>mlee@school.example</td></tr></table><a href=\"/coaches\">Coaches</a>";
    let second = b"<table><tr><th>Sport</th><th>Coach</th><th>E-mail</th></tr><tr><td>Track - Boys</td><td>John Smith</td><td>jsmith@school.example</td></tr></table>";
    cache(
        &fetcher,
        "https://school.example/start",
        "https://school.example/staff",
        first,
        200,
    )?;
    cache(
        &fetcher,
        "https://school.example/coaches",
        "https://school.example/start",
        second,
        200,
    )?;
    let row = QueueRow {
        state: "MI".to_owned(),
        name: "School".to_owned(),
        website: "https://school.example/start".to_owned(),
    };
    let outcome = crawl_site(&Rules::new()?, &fetcher, &row, false).await?;
    check!(outcome
        .signals
        .coach_hits
        .iter()
        .any(|hit| hit.name == "Mary Ann Lee" && hit.url == "https://school.example/staff"));
    check!(outcome
        .signals
        .coach_hits
        .iter()
        .any(|hit| hit.name == "John Smith" && hit.url == "https://school.example/start"));
    for hit in &outcome.signals.coach_hits {
        check!(outcome.pages.iter().any(|page| page.url == hit.url));
    }
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn fetch_admits_exact_school_page_budget_and_refuses_one_more_byte() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = Fetcher::new(
        dir.path(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["school.example".to_owned()],
    )?
    .with_offline(true);
    cache(
        &fetcher,
        "https://school.example/robots.txt",
        "https://school.example/robots.txt",
        b"",
        200,
    )?;
    let prefix = "<p>Head Track Coach Jane Doe</p><!--";
    let padding = crate::school_sites::MAX_PAGE_BYTES
        .checked_sub(prefix.len())
        .and_then(|size| size.checked_sub(3))
        .ok_or("fixture padding")?;
    let mut body = format!("{prefix}{}-->", "x".repeat(padding));
    cache(
        &fetcher,
        "https://school.example/exact",
        "https://school.example/exact",
        body.as_bytes(),
        200,
    )?;
    body.push(' ');
    cache(
        &fetcher,
        "https://school.example/over",
        "https://school.example/over",
        body.as_bytes(),
        200,
    )?;
    let row = QueueRow {
        state: "MI".to_owned(),
        name: "School".to_owned(),
        website: "https://school.example/exact".to_owned(),
    };
    let outcome = crawl_site(&Rules::new()?, &fetcher, &row, false).await?;
    check!(eq; outcome.pages.len(), 1);
    let row = QueueRow {
        website: "https://school.example/over".to_owned(),
        ..row
    };
    check!(
        matches!(crawl_site(&Rules::new()?, &fetcher, &row, false).await,
        Err(crate::CrawlError::Resource { requested, limit, .. })
            if requested == crate::school_sites::MAX_PAGE_BYTES + 1 && limit == crate::school_sites::MAX_PAGE_BYTES)
    );
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}
