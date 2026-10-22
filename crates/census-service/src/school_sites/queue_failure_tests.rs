use super::super::{artifacts::SiteRecord, research, run, SchoolSitesArgs};
use census_crawl::net::Fetcher;
use census_domain::model::CanonicalSchool;
use census_store::{Store, Table};
use std::collections::HashMap;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ROOT: &str = "https://school.org";

#[tokio::test]
async fn malformed_first_school_retains_failure_and_valid_next_school_artifact() -> TestResult {
    queue_retains_next_school(
        b"<template><a href='/must-not-follow'>Coaches</a>",
        "https://school.org/bad: schema:",
    )
    .await
}

#[tokio::test]
async fn exhausted_site_diagnostics_preserve_next_school_artifact() -> TestResult {
    let html = format!("<a href='//[{}'>Coaches</a>", "a".repeat(4000)).repeat(400);
    queue_retains_next_school(html.as_bytes(), "diagnostic_budget_exhausted:").await
}

#[tokio::test]
async fn oversized_first_school_retains_failure_and_valid_next_school_artifact() -> TestResult {
    let body = vec![b' '; 8 * 1024 * 1024 + 1];
    queue_retains_next_school(&body, "resource: school site body bytes").await
}

#[tokio::test]
async fn non_utf8_first_school_retains_failure_and_valid_next_school_artifact() -> TestResult {
    queue_retains_next_school(&[0xff], "schema: school site is not UTF-8").await
}

#[tokio::test]
async fn token_limited_first_school_does_not_leak_mailbox_prefix() -> TestResult {
    let body = format!(
        "<p>School office <a href='mailto:leak@school.example'>leak@school.example</a></p>{}",
        "<span></span>".repeat(33_000)
    );
    queue_retains_next_school(body.as_bytes(), "resource: school mailbox candidate page parser budget exhausted: school contact page exceeds 65,536 HTML tokens").await
}

async fn queue_retains_next_school(first: &[u8], marker: &str) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["school.org".to_owned()],
    )?
    .with_offline(true);
    seed(fetcher.cache_dir(), first)?;
    let input = dir.path().join("queue.jsonl");
    let queue = [
        serde_json::json!({"state":"MI","name":"Bad","website":format!("{ROOT}/bad")}),
        serde_json::json!({"state":"MI","name":"Good","website":format!("{ROOT}/good")}),
    ]
    .into_iter()
    .map(|row| row.to_string())
    .collect::<Vec<_>>()
    .join("\n");
    std::fs::write(&input, queue)?;
    let out = dir.path().join("out");
    let args = SchoolSitesArgs {
        input,
        out: Some(out.clone()),
        state: None,
        limit: None,
        sample: None,
        refresh: false,
        authorize_queue_hosts: false,
    };
    let report = run(&fetcher, &args, &store).await?;
    check!(eq; (report.planned, report.crawled, report.failed, report.empty), (2, 2, 1, 0));
    let bad: SiteRecord = serde_json::from_slice(&std::fs::read(out.join("MI__bad.json"))?)?;
    check!(eq; bad.research_status, research::ResearchStatus::Incomplete);
    let note = bad.note.as_deref().ok_or("retained diagnostic")?;
    check!(note.contains(marker));
    check!(note.starts_with("https://school.org/bad:"));
    check!(note.len() <= 1024 * 1024);
    check!(bad.school_mailbox_candidates.is_empty());
    check!(bad.staff_contact_candidates.is_empty());
    check!(eq; bad.requests, 1);
    check!(eq; bad.page_evidence.len(), 1);
    let good: SiteRecord = serde_json::from_slice(&std::fs::read(out.join("MI__good.json"))?)?;
    check!(eq; good.research_status, research::ResearchStatus::UnverifiedCandidates);
    check!(good
        .school_mailbox_candidates
        .iter()
        .any(|hit| hit.mailbox == "office@school.org"));
    check!(store.scan::<CanonicalSchool>(Table::Schools)?.is_empty());
    check!(out.join("report.json").is_file());
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

fn seed(cache: &std::path::Path, first: &[u8]) -> TestResult {
    let now = census_crawl::net::now_iso8601();
    let good = b"<p>School office <a href='mailto:office@school.org'>office@school.org</a></p>";
    for (path, body) in [
        ("/bad", first),
        ("/good", good.as_slice()),
        ("/athletics", b"<p>No contact listing</p>".as_slice()),
        ("/coaches", b"<p>No contact listing</p>".as_slice()),
        ("/staff-directory", b"<p>No contact listing</p>".as_slice()),
    ] {
        crate::capture_cache::seed_response(
            cache,
            &format!("{ROOT}{path}"),
            body,
            &now,
            crate::capture_cache::Metadata::default(),
        )?;
    }
    Ok(())
}
