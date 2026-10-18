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
    let outcome = crawl_site(&Rules::new()?, &fetcher, &row, false).await?;
    check!(eq; (outcome.errors, outcome.pages.len()), (1, 1));
    check!(outcome.note.as_deref().is_some_and(
        |note| note.contains("https://school.example/over: resource: school site body bytes")
    ));
    check!(outcome.signals.pages.is_empty());
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn malformed_page_keeps_error_and_capture_then_continues_admitted_pages() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = Fetcher::new(
        dir.path(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["school.org".to_owned()],
    )?
    .with_offline(true);
    cache(
        &fetcher,
        "https://school.org/robots.txt",
        "https://school.org/robots.txt",
        b"",
        200,
    )?;
    let home = b"<p>Head Track Coach Jane Doe</p><a href='/bad'>Coaches</a><a href='/good'>Coaches</a><a href='/missing'>Coaches</a>";
    let good = b"<p>School office <a href='mailto:office@school.org'>office@school.org</a></p>";
    for (url, body) in [
        ("https://school.org/", home.as_slice()),
        ("https://school.org/bad", b"<template>".as_slice()),
        ("https://school.org/good", good.as_slice()),
    ] {
        cache(&fetcher, url, url, body, 200)?;
    }
    let row = QueueRow {
        state: "MI".to_owned(),
        name: "School".to_owned(),
        website: "https://school.org/".to_owned(),
    };
    let outcome = crawl_site(&Rules::new()?, &fetcher, &row, false).await?;
    check!(eq; outcome.errors, 2);
    let note = outcome.note.as_deref().ok_or("retained failures")?;
    check!(note.contains("https://school.org/bad: schema:"));
    check!(note.contains("https://school.org/missing: fetch:"));
    check!(outcome
        .pages
        .iter()
        .any(|page| page.url == "https://school.org/bad"));
    check!(!outcome
        .signals
        .pages
        .iter()
        .any(|url| url == "https://school.org/bad"));
    check!(outcome
        .signals
        .school_mailboxes
        .iter()
        .any(|hit| hit.email == "office@school.org"));
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn undecodable_home_retains_capture_and_stops_that_site_only() -> TestResult {
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
    cache(
        &fetcher,
        "https://school.example/",
        "https://school.example/",
        &[0xff],
        200,
    )?;
    let row = QueueRow {
        state: "MI".to_owned(),
        name: "School".to_owned(),
        website: "https://school.example/".to_owned(),
    };
    let outcome = crawl_site(&Rules::new()?, &fetcher, &row, false).await?;
    check!(eq; (outcome.requests, outcome.errors, outcome.pages.len()), (1, 1, 1));
    check!(outcome
        .note
        .as_deref()
        .is_some_and(|note| note.contains("schema: school site is not UTF-8")));
    check!(outcome.signals.pages.is_empty());
    Ok(())
}
