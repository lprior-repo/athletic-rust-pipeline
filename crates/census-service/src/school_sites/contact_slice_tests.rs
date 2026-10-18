use crate::school_sites::{artifacts::SiteRecord, research, run, SchoolSitesArgs};
use census_crawl::net::Fetcher;
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachRole, Gender};
use census_store::{Store, Table};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ROOT: &str = "https://lotusschool.org";
const DIRECTOR: &str = "https://lotusschool.org/athletics-dpt/";
const AD_BODY: &[u8] = br#"<ul class='profile-list-box'><li>Deon Jackson</li><li>Athletic Director</li><li><a href='mailto:djackson@lotusschool.org'>djackson@lotusschool.org</a></li></ul>"#;
const HOME: &[u8] = br#"<nav>
<a href='/athletics/football'>Football</a>
<a href='/athletics/baseball'>Baseball</a>
<a href='/athletics/basketball'>Basketball</a>
<a href='/athletics/volleyball'>Volleyball</a>
<a href='/athletics/cheerleading'>Cheerleading</a>
<a href='/athletics/dance'>Dance</a>
<a href='/cross-country'>Cross Country</a>
<a href='/athletics-dpt/'>Contact Athletic Director</a>
<a href='/activities'>Activities</a>
</nav>"#;

#[tokio::test]
async fn contact_slice_survives_bad_school_and_retains_only_scoped_ad_artifact() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store_path = dir.path().join("store");
    let store = Store::open(&store_path)?;
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["lotusschool.org".to_owned()],
    )?
    .with_offline(true);
    let acquired_at = census_crawl::net::now_iso8601();
    seed(fetcher.cache_dir(), &acquired_at)?;
    let input = dir.path().join("queue.jsonl");
    let rows = [
        serde_json::json!({"state":"CO","name":"Bad","website":format!("{ROOT}/bad")}),
        serde_json::json!({"state":"CO","name":"Lotus","website":format!("{ROOT}/good")}),
    ]
    .into_iter()
    .map(|row| row.to_string())
    .collect::<Vec<_>>()
    .join("\n");
    std::fs::write(&input, rows)?;
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
    check!(eq; (report.emails, report.coach_contacts, report.ad_contacts), (0, 0, 0));
    check!(eq; report.requests, 5);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    drop(store);
    let reopened = Store::open(&store_path)?;
    check!(reopened.scan::<CanonicalSchool>(Table::Schools)?.is_empty());
    check!(reopened.scan::<CanonicalCoach>(Table::Coaches)?.is_empty());
    verify_artifacts(&out, &acquired_at)?;
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("report.json"))?)?;
    for (field, value) in [
        ("planned", 2),
        ("crawled", 2),
        ("failed", 1),
        ("empty", 0),
        ("requests", 5),
        ("emails", 0),
        ("coach_contacts", 0),
        ("ad_contacts", 0),
    ] {
        check!(eq; saved.get(field), Some(&serde_json::json!(value)));
    }
    Ok(())
}

fn verify_artifacts(out: &Path, acquired_at: &str) -> TestResult {
    let bad: SiteRecord = serde_json::from_slice(&std::fs::read(out.join("CO__bad.json"))?)?;
    check!(eq; bad.research_status, research::ResearchStatus::Incomplete);
    check!(bad
        .note
        .as_deref()
        .is_some_and(|note| note.starts_with("https://lotusschool.org/bad: schema:")));
    check!(bad.staff_contact_candidates.is_empty());
    check!(bad.school_mailbox_candidates.is_empty());
    check!(eq; (bad.requests, bad.page_evidence.len()), (1, 1));
    let good: SiteRecord = serde_json::from_slice(&std::fs::read(out.join("CO__lotus.json"))?)?;
    check!(eq; good.coach_research_status, research::CoachResearchStatus::UnverifiedCandidates);
    check!(eq; (good.requests, good.errors), (4, 0));
    check!(good.note.is_none());
    let pages: Vec<_> = good
        .page_evidence
        .iter()
        .map(|page| page.url.as_str())
        .collect();
    check!(eq; pages, vec!["https://lotusschool.org/good", "https://lotusschool.org/cross-country", DIRECTOR, "https://lotusschool.org/activities"]);
    check!(eq; good.staff_contact_candidates.len(), 1);
    let candidate = good
        .staff_contact_candidates
        .first()
        .ok_or("published AD candidate")?;
    check!(eq; candidate.staff.name.as_str(), "Deon Jackson");
    check!(eq; candidate.staff.published_role.as_str(), "Athletic Director");
    check!(eq; candidate.staff.role, CoachRole::AthleticDirector);
    check!(eq; candidate.staff.email.as_str(), "djackson@lotusschool.org");
    check!(eq; candidate.staff.sport, None);
    check!(eq; candidate.staff.gender, Gender::Unknown);
    check!(eq; candidate.staff.url.as_str(), DIRECTOR);
    check!(eq; candidate.source.url.as_str(), DIRECTOR);
    check!(eq; candidate.source.digest, format!("{:x}", Sha256::digest(AD_BODY)));
    check!(eq; candidate.source.fetched_at.as_str(), acquired_at);
    check!(eq; candidate.source.status, 200);
    check!(good.page_evidence.contains(&candidate.source));
    Ok(())
}

fn seed(cache: &Path, acquired_at: &str) -> TestResult {
    for (path, body) in [
        ("/robots.txt", b"User-agent: *\nAllow: /\n".as_slice()),
        (
            "/bad",
            b"<template><a href='/must-not-follow'>Coaches</a>".as_slice(),
        ),
        ("/good", HOME),
        ("/cross-country", b"<p>Cross Country</p>".as_slice()),
        ("/athletics-dpt/", AD_BODY),
        ("/activities", b"<p>Activities</p>".as_slice()),
        (
            "/athletics/football",
            b"<p>Football sentinel</p>".as_slice(),
        ),
        (
            "/athletics/baseball",
            b"<p>Baseball sentinel</p>".as_slice(),
        ),
        (
            "/athletics/basketball",
            b"<p>Basketball sentinel</p>".as_slice(),
        ),
        (
            "/athletics/volleyball",
            b"<p>Volleyball sentinel</p>".as_slice(),
        ),
        (
            "/athletics/cheerleading",
            b"<p>Cheerleading sentinel</p>".as_slice(),
        ),
        ("/athletics/dance", b"<p>Dance sentinel</p>".as_slice()),
    ] {
        crate::capture_cache::seed_response(
            cache,
            &format!("{ROOT}{path}"),
            body,
            acquired_at,
            crate::capture_cache::Metadata::default(),
        )?;
    }
    Ok(())
}
